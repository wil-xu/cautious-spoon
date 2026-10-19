use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;

use crate::data;
use crate::data::Comment;
use crate::data::Conversation;
use crate::data::Job;
use crate::data::Message;
use crate::data::News;
use crate::data::Notification;
use crate::data::Person;
use crate::data::Post;
use crate::data::Profile;
use crate::data::Suggestion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Home,
    Network,
    Jobs,
    Messaging,
    Notifications,
}

impl Tab {
    pub(crate) const ALL: [Tab; 5] = [
        Tab::Home,
        Tab::Network,
        Tab::Jobs,
        Tab::Messaging,
        Tab::Notifications,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Network => "My Network",
            Tab::Jobs => "Jobs",
            Tab::Messaging => "Messaging",
            Tab::Notifications => "Notifications",
        }
    }

    fn index(self) -> usize {
        Tab::ALL
            .iter()
            .position(|&t| t == self)
            .expect("every tab is listed in Tab::ALL")
    }

    fn cycle(self, forward: bool) -> Tab {
        let n = Tab::ALL.len();
        let i = self.index();
        Tab::ALL[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Moving through the current tab's list.
    Browse,
    /// Home: the selected post with its comments. `comment` is the selected
    /// comment.
    PostDetail { comment: usize },
    /// Home: the post view with its "Add a comment" box focused. `comment` is
    /// the comment that was selected, which Esc selects again. Swallows
    /// printable keys.
    CommentBox { comment: usize },
    /// Home: the "Start a post" composer overlay. Swallows printable keys.
    Composer,
    /// Messaging: the reply box under the open thread. Swallows printable keys.
    Reply,
}

/// The search overlay, open over whatever tab and mode it was opened from.
/// Those stay untouched underneath, so closing it puts everything back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Search {
    pub query: String,
    /// Index into `App::search_hits`.
    pub selected: usize,
}

/// One search result, pointing into the list it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchHit {
    Invitation(usize),
    Suggestion(usize),
    Job(usize),
    Post(usize),
}

/// What a search query matches against: trimmed and lowercased. The same
/// lowercasing goes on every searched field, so case never matters.
pub(crate) fn search_needle(query: &str) -> String {
    query.trim().to_lowercase()
}

/// The selected row on My Network, which spans both sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkItem {
    Invitation(usize),
    Suggestion(usize),
}

pub(crate) struct App {
    pub profile: Profile,
    pub posts: Vec<Post>,
    pub news: Vec<News>,
    pub invitations: Vec<Person>,
    pub suggestions: Vec<Suggestion>,
    pub jobs: Vec<Job>,
    pub conversations: Vec<Conversation>,
    pub notifications: Vec<Notification>,
    pub tab: Tab,
    pub mode: Mode,
    /// Index into `posts`.
    pub post_sel: usize,
    /// Index into invitations followed by suggestions, as one list.
    pub network_sel: usize,
    pub job_sel: usize,
    pub chat_sel: usize,
    pub notif_sel: usize,
    /// The conversation shown on the right of Messaging, opened with Enter.
    pub open_chat: Option<usize>,
    /// Composer text.
    pub draft: String,
    /// Reply box text.
    pub reply: String,
    /// The post view's comment box text.
    pub comment_draft: String,
    /// `Some` while the search overlay is open. Takes every key but Ctrl-C.
    pub search: Option<Search>,
    pub status: Option<&'static str>,
    pub should_quit: bool,
}

/// A key that should be typed into a text box: printable, with no Ctrl or Alt.
fn typed(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(ch)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(ch)
        }
        _ => None,
    }
}

impl App {
    pub(crate) fn new() -> Self {
        App {
            profile: data::profile(),
            posts: data::posts(),
            news: data::news(),
            invitations: data::invitations(),
            suggestions: data::suggestions(),
            jobs: data::jobs(),
            conversations: data::conversations(),
            notifications: data::notifications(),
            tab: Tab::Home,
            mode: Mode::Browse,
            post_sel: 0,
            network_sel: 0,
            job_sel: 0,
            chat_sel: 0,
            notif_sel: 0,
            open_chat: None,
            draft: String::new(),
            reply: String::new(),
            comment_draft: String::new(),
            search: None,
            status: None,
            should_quit: false,
        }
    }

    /// The count shown next to a tab label. Zero means no badge.
    pub(crate) fn badge(&self, tab: Tab) -> usize {
        match tab {
            Tab::Home | Tab::Jobs => 0,
            Tab::Network => self.invitations.len(),
            Tab::Messaging => self.conversations.iter().filter(|c| c.unread).count(),
            Tab::Notifications => self.notifications.iter().filter(|n| n.unread).count(),
        }
    }

    pub(crate) fn saved_jobs(&self) -> usize {
        self.jobs.iter().filter(|j| j.saved).count()
    }

    pub(crate) fn selected_post(&self) -> Option<&Post> {
        self.posts.get(self.post_sel)
    }

    pub(crate) fn network_item(&self) -> Option<NetworkItem> {
        let i = self.network_sel;
        let n = self.invitations.len();
        if i < n {
            Some(NetworkItem::Invitation(i))
        } else if i - n < self.suggestions.len() {
            Some(NetworkItem::Suggestion(i - n))
        } else {
            None
        }
    }

    /// Everything that matches `query`, from the current state: people
    /// (invitations, then suggestions), then jobs, then posts in feed order.
    /// A query that is empty once trimmed matches nothing.
    pub(crate) fn search_hits(&self, query: &str) -> Vec<SearchHit> {
        let needle = search_needle(query);
        if needle.is_empty() {
            return Vec::new();
        }
        let found = |fields: &[&str]| fields.iter().any(|f| f.to_lowercase().contains(&needle));
        let mut hits = Vec::new();
        for (i, p) in self.invitations.iter().enumerate() {
            if found(&[p.name, p.headline]) {
                hits.push(SearchHit::Invitation(i));
            }
        }
        for (i, s) in self.suggestions.iter().enumerate() {
            if found(&[s.person.name, s.person.headline]) {
                hits.push(SearchHit::Suggestion(i));
            }
        }
        for (i, job) in self.jobs.iter().enumerate() {
            if found(&[job.title, job.company, job.location]) {
                hits.push(SearchHit::Job(i));
            }
        }
        for (i, post) in self.posts.iter().enumerate() {
            if found(&[post.author, post.headline, &post.body]) {
                hits.push(SearchHit::Post(i));
            }
        }
        hits
    }

    fn network_len(&self) -> usize {
        self.invitations.len() + self.suggestions.len()
    }

    /// Switching tabs closes any overlay. Opening Notifications marks them all
    /// read, which clears the badge, but the ones that were unread stay marked
    /// as new until you leave the tab.
    fn set_tab(&mut self, tab: Tab) {
        let entering = tab == Tab::Notifications && self.tab != Tab::Notifications;
        let leaving = self.tab == Tab::Notifications && tab != Tab::Notifications;
        self.tab = tab;
        self.mode = Mode::Browse;
        for n in &mut self.notifications {
            if entering {
                n.is_new = n.unread;
                n.unread = false;
            } else if leaving {
                n.is_new = false;
            }
        }
    }

    fn move_selection(&mut self, delta: isize) {
        fn step(sel: &mut usize, len: usize, delta: isize) {
            *sel = sel.saturating_add_signed(delta).min(len.saturating_sub(1));
        }
        match (self.tab, self.mode) {
            (Tab::Home, Mode::PostDetail { comment }) => {
                let len = self.selected_post().map_or(0, |p| p.comments.len());
                let mut comment = comment;
                step(&mut comment, len, delta);
                self.mode = Mode::PostDetail { comment };
            }
            (Tab::Home, _) => step(&mut self.post_sel, self.posts.len(), delta),
            (Tab::Network, _) => {
                let len = self.network_len();
                step(&mut self.network_sel, len, delta);
            }
            (Tab::Jobs, _) => step(&mut self.job_sel, self.jobs.len(), delta),
            (Tab::Messaging, _) => step(&mut self.chat_sel, self.conversations.len(), delta),
            (Tab::Notifications, _) => step(&mut self.notif_sel, self.notifications.len(), delta),
        }
    }

    pub(crate) fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }
        self.status = None;
        if self.search.is_some() {
            self.on_search_key(key);
            return;
        }
        match self.mode {
            Mode::Composer => self.on_composer_key(key),
            Mode::Reply => self.on_reply_key(key),
            Mode::CommentBox { comment } => self.on_comment_key(key, comment),
            Mode::Browse | Mode::PostDetail { .. } => self.on_browse_key(key.code),
        }
    }

    fn on_browse_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char(ch @ '1'..='5') => self.set_tab(Tab::ALL[usize::from(ch as u8 - b'1')]),
            KeyCode::Tab => self.set_tab(self.tab.cycle(true)),
            KeyCode::BackTab => self.set_tab(self.tab.cycle(false)),
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Esc => self.mode = Mode::Browse,
            KeyCode::Char('/') => self.search = Some(Search::default()),
            _ => match self.tab {
                Tab::Home => self.on_home_key(code),
                Tab::Network => self.on_network_key(code),
                Tab::Jobs => self.on_jobs_key(code),
                Tab::Messaging => self.on_messaging_key(code),
                Tab::Notifications => {}
            },
        }
    }

    /// The search box is a text box, like the composer, except that Ctrl- and
    /// Alt-modified keys do nothing at all.
    fn on_search_key(&mut self, key: KeyEvent) {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        let hits = match &self.search {
            Some(search) => self.search_hits(&search.query),
            None => return,
        };
        let Some(search) = self.search.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.search = None,
            KeyCode::Enter => match hits.get(search.selected) {
                Some(&hit) => self.open_hit(hit),
                None => self.status = Some("No results"),
            },
            KeyCode::Up => search.selected = search.selected.saturating_sub(1),
            KeyCode::Down => {
                search.selected = (search.selected + 1).min(hits.len().saturating_sub(1));
            }
            KeyCode::Backspace => {
                if search.query.pop().is_some() {
                    search.selected = 0;
                }
            }
            _ => {
                if let Some(ch) = typed(key) {
                    search.query.push(ch);
                    search.selected = 0;
                }
            }
        }
    }

    /// Closes search and goes to `hit`, switching tabs the way `1`-`5` do.
    fn open_hit(&mut self, hit: SearchHit) {
        self.search = None;
        match hit {
            SearchHit::Invitation(i) => {
                self.set_tab(Tab::Network);
                self.network_sel = i;
            }
            SearchHit::Suggestion(i) => {
                self.set_tab(Tab::Network);
                self.network_sel = self.invitations.len() + i;
            }
            SearchHit::Job(i) => {
                self.set_tab(Tab::Jobs);
                self.job_sel = i;
            }
            SearchHit::Post(i) => {
                self.set_tab(Tab::Home);
                self.post_sel = i;
            }
        }
    }

    fn on_home_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('l') => {
                if let Some(post) = self.posts.get_mut(self.post_sel) {
                    post.liked = !post.liked;
                    if post.liked {
                        post.reactions += 1;
                    } else {
                        post.reactions -= 1;
                    }
                }
            }
            KeyCode::Enter if self.mode == Mode::Browse && !self.posts.is_empty() => {
                self.mode = Mode::PostDetail { comment: 0 };
            }
            KeyCode::Char('p') if self.mode == Mode::Browse => self.mode = Mode::Composer,
            KeyCode::Char('c') => {
                if let Mode::PostDetail { comment } = self.mode {
                    self.comment_draft.clear();
                    self.mode = Mode::CommentBox { comment };
                }
            }
            KeyCode::Char('x') => {
                if let Mode::PostDetail { comment } = self.mode {
                    self.delete_comment(comment);
                }
            }
            _ => {}
        }
    }

    /// Removes comment `index` of the open post if it's yours. The selection
    /// keeps its index, clamped to the list, so the next comment slides under
    /// the cursor, or the cursor moves up when the last one goes. Anyone
    /// else's comment, or no comment at all, is left alone without a toast.
    fn delete_comment(&mut self, index: usize) {
        let Some(post) = self.posts.get_mut(self.post_sel) else {
            return;
        };
        if post
            .comments
            .get(index)
            .is_none_or(|c| c.author != self.profile.name)
        {
            return;
        }
        post.comments.remove(index);
        self.mode = Mode::PostDetail {
            comment: index.min(post.comments.len().saturating_sub(1)),
        };
        self.status = Some("Comment deleted");
    }

    /// The comment box is a text box, like search: Ctrl- and Alt-modified
    /// keys do nothing at all. `comment` is the comment selected before `c`.
    fn on_comment_key(&mut self, key: KeyEvent, comment: usize) {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.comment_draft.clear();
                self.mode = Mode::PostDetail { comment };
            }
            KeyCode::Enter => self.post_comment(),
            KeyCode::Backspace => {
                self.comment_draft.pop();
            }
            _ => {
                if let Some(ch) = typed(key) {
                    self.comment_draft.push(ch);
                }
            }
        }
    }

    /// Appends the comment box text to the open post as the newest comment,
    /// and selects it.
    fn post_comment(&mut self) {
        let body = self.comment_draft.trim();
        if body.is_empty() {
            self.status = Some("Write something first");
            return;
        }
        let Some(post) = self.posts.get_mut(self.post_sel) else {
            return;
        };
        post.comments.push(Comment {
            author: self.profile.name,
            headline: self.profile.headline,
            hours_ago: 0,
            body: body.to_owned(),
        });
        self.mode = Mode::PostDetail {
            comment: post.comments.len() - 1,
        };
        self.comment_draft.clear();
        self.status = Some("Comment posted");
    }

    fn on_composer_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.draft.clear();
                self.mode = Mode::Browse;
            }
            KeyCode::Enter => self.publish(),
            KeyCode::Backspace => {
                self.draft.pop();
            }
            _ => {
                if let Some(ch) = typed(key) {
                    self.draft.push(ch);
                }
            }
        }
    }

    fn publish(&mut self) {
        let body = self.draft.trim();
        if body.is_empty() {
            self.status = Some("Write something first");
            return;
        }
        self.posts.insert(
            0,
            Post {
                author: self.profile.name,
                headline: self.profile.headline,
                hours_ago: 0,
                body: body.to_owned(),
                reactions: 0,
                comments: Vec::new(),
                liked: false,
            },
        );
        self.draft.clear();
        self.post_sel = 0;
        self.mode = Mode::Browse;
        self.status = Some("Post published");
    }

    fn on_network_key(&mut self, code: KeyCode) {
        let item = self.network_item();
        match (code, item) {
            (KeyCode::Char('a'), Some(NetworkItem::Invitation(i))) => {
                self.invitations.remove(i);
                self.profile.connections += 1;
                self.status = Some("Invitation accepted");
            }
            (KeyCode::Char('x'), Some(NetworkItem::Invitation(i))) => {
                self.invitations.remove(i);
                self.status = Some("Invitation ignored");
            }
            (KeyCode::Char('c'), Some(NetworkItem::Suggestion(i))) => {
                let s = &mut self.suggestions[i];
                s.pending = !s.pending;
                self.status = Some(if s.pending {
                    "Invitation sent"
                } else {
                    "Invitation withdrawn"
                });
            }
            _ => {}
        }
        // The removed row's successor slides into place under the cursor.
        self.network_sel = self.network_sel.min(self.network_len().saturating_sub(1));
    }

    fn on_jobs_key(&mut self, code: KeyCode) {
        let Some(job) = self.jobs.get_mut(self.job_sel) else {
            return;
        };
        match code {
            KeyCode::Char('s') => {
                job.saved = !job.saved;
                self.status = Some(if job.saved {
                    "Job saved"
                } else {
                    "Job unsaved"
                });
            }
            KeyCode::Char('e') if job.applied => self.status = Some("Already applied"),
            KeyCode::Char('e') => {
                job.applied = true;
                self.status = Some("Application sent");
            }
            _ => {}
        }
    }

    fn on_messaging_key(&mut self, code: KeyCode) {
        if code == KeyCode::Enter
            && let Some(chat) = self.conversations.get_mut(self.chat_sel)
        {
            chat.unread = false;
            self.open_chat = Some(self.chat_sel);
            self.reply.clear();
            self.mode = Mode::Reply;
        }
    }

    fn on_reply_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.reply.clear();
                self.mode = Mode::Browse;
            }
            KeyCode::Enter => {
                let body = self.reply.trim();
                if let Some(chat) = self.open_chat.and_then(|i| self.conversations.get_mut(i))
                    && !body.is_empty()
                {
                    chat.messages.push(Message {
                        from_me: true,
                        body: body.to_owned(),
                        hours_ago: 0,
                    });
                    self.reply.clear();
                }
            }
            KeyCode::Backspace => {
                self.reply.pop();
            }
            _ => {
                if let Some(ch) = typed(key) {
                    self.reply.push(ch);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_str(app: &mut App, s: &str) {
        for ch in s.chars() {
            press(app, KeyCode::Char(ch));
        }
    }

    fn query(app: &App) -> Option<&str> {
        app.search.as_ref().map(|s| s.query.as_str())
    }

    fn selected_hit(app: &App) -> usize {
        app.search.as_ref().expect("search is open").selected
    }

    /// Everything that `/` then `Esc` must leave as it was.
    type Snapshot = (Tab, Mode, [usize; 5], Option<usize>, Vec<bool>);

    fn snapshot(app: &App) -> Snapshot {
        (
            app.tab,
            app.mode,
            [
                app.post_sel,
                app.network_sel,
                app.job_sel,
                app.chat_sel,
                app.notif_sel,
            ],
            app.open_chat,
            app.notifications.iter().map(|n| n.is_new).collect(),
        )
    }

    /// The kind and name of each hit, to compare against the seed.
    fn named_hits(app: &App, query: &str) -> Vec<(&'static str, String)> {
        app.search_hits(query)
            .into_iter()
            .map(|hit| match hit {
                SearchHit::Invitation(i) => ("invitation", app.invitations[i].name.to_owned()),
                SearchHit::Suggestion(i) => {
                    ("suggestion", app.suggestions[i].person.name.to_owned())
                }
                SearchHit::Job(i) => (
                    "job",
                    format!("{} at {}", app.jobs[i].title, app.jobs[i].company),
                ),
                SearchHit::Post(i) => ("post", app.posts[i].author.to_owned()),
            })
            .collect()
    }

    /// Opens the post view of Maren Holt's post, the first in the feed.
    fn maren_post_view() -> App {
        let mut app = App::new();
        assert_eq!(app.posts[0].author, "Maren Holt");
        press(&mut app, KeyCode::Enter);
        app
    }

    #[test]
    fn c_focuses_the_comment_box_only_in_the_post_view() {
        // The feed: nothing.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.mode, Mode::Browse);
        assert_eq!(app.post_sel, 1);
        assert!(app.status.is_none());
        assert!(app.comment_draft.is_empty());

        // My Network: still Connect / Pending.
        press(&mut app, KeyCode::Char('2'));
        for _ in 0..3 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Char('c'));
        assert!(app.suggestions[0].pending);
        assert_eq!(app.status, Some("Invitation sent"));
        assert_eq!(app.mode, Mode::Browse);
        press(&mut app, KeyCode::Char('c'));
        assert!(!app.suggestions[0].pending);
        assert_eq!(app.status, Some("Invitation withdrawn"));

        // The post view, with the third comment selected.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.mode, Mode::CommentBox { comment: 2 });
        assert!(app.status.is_none());

        // Esc discards the draft and selects the same comment again.
        let n = app.posts[0].comments.len();
        type_str(&mut app, "never mind");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        assert!(app.comment_draft.is_empty());
        assert_eq!(app.posts[0].comments.len(), n);

        // Every visit starts empty.
        press(&mut app, KeyCode::Char('c'));
        assert!(app.comment_draft.is_empty());
    }

    #[test]
    fn the_comment_box_types_keys_instead_of_acting_on_them() {
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('c'));
        let before = snapshot(&app);
        let reactions = app.posts[0].reactions;
        let comments = app.posts[0].comments.len();

        type_str(&mut app, "q1jklcp/ 5");
        assert_eq!(app.comment_draft, "q1jklcp/ 5");
        assert!(!app.should_quit, "q is typed, not quit");
        assert_eq!(snapshot(&app), before, "1, 5, j and k are typed");
        assert_eq!(app.mode, Mode::CommentBox { comment: 1 });
        assert_eq!(app.posts[0].reactions, reactions, "l is typed, not a like");
        assert!(!app.posts[0].liked);
        assert!(app.search.is_none(), "/ is typed, not search");
        assert!(app.draft.is_empty(), "p is typed, not the composer");
        assert_eq!(app.posts[0].comments.len(), comments);
        assert!(app.status.is_none());

        // Shift is fine.
        app.on_key(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT));
        assert_eq!(app.comment_draft, "q1jklcp/ 5Q");

        // Tab, BackTab, arrows and Ctrl- or Alt-modified keys do nothing.
        for code in [
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
        ] {
            press(&mut app, code);
        }
        for (code, modifiers) in [
            (KeyCode::Char('a'), KeyModifiers::CONTROL),
            (KeyCode::Char('l'), KeyModifiers::ALT),
            (KeyCode::Backspace, KeyModifiers::CONTROL),
            (KeyCode::Backspace, KeyModifiers::ALT),
            (KeyCode::Enter, KeyModifiers::CONTROL),
            (KeyCode::Enter, KeyModifiers::ALT),
            (KeyCode::Esc, KeyModifiers::CONTROL),
            (KeyCode::Char('1'), KeyModifiers::ALT),
        ] {
            app.on_key(KeyEvent::new(code, modifiers));
        }
        assert_eq!(app.comment_draft, "q1jklcp/ 5Q");
        assert_eq!(snapshot(&app), before);
        assert_eq!(app.posts[0].comments.len(), comments);
        assert!(app.status.is_none());

        // Backspace deletes, and on an empty draft keeps focus.
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.comment_draft, "q1jklcp/ 5");
        for _ in 0..20 {
            press(&mut app, KeyCode::Backspace);
        }
        assert!(app.comment_draft.is_empty());
        assert_eq!(app.mode, Mode::CommentBox { comment: 1 }, "still focused");

        // Ctrl-C still quits.
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn posting_a_comment_appends_it_and_selects_it() {
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        let reactions = app.posts[0].reactions;
        let others: Vec<usize> = app.posts.iter().map(|p| p.comments.len()).collect();
        assert_eq!(app.posts[0].comments.len(), 3);
        let before = snapshot(&app);

        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "  Great write-up,  thanks! q1/  ");
        press(&mut app, KeyCode::Enter);

        let post = &app.posts[0];
        assert_eq!(post.comments.len(), 4);
        let mine = post.comments.last().expect("just posted");
        assert_eq!(mine.author, "Jordan Vale");
        assert_eq!(mine.headline, app.profile.headline);
        assert_eq!(mine.hours_ago, 0);
        assert_eq!(mine.body, "Great write-up,  thanks! q1/", "trimmed only");
        // The seed comments are untouched, oldest first.
        assert_eq!(post.comments[0].author, data::posts()[0].comments[0].author);
        assert_eq!(app.mode, Mode::PostDetail { comment: 3 });
        assert!(app.comment_draft.is_empty());
        assert_eq!(app.status, Some("Comment posted"));

        // Nothing else moved.
        assert_eq!(post.reactions, reactions);
        assert!(!post.liked);
        assert_eq!(app.post_sel, 0);
        let (tab, _, sels, open_chat, new) = snapshot(&app);
        assert_eq!(
            (tab, sels, open_chat, new),
            (before.0, before.2, before.3, before.4)
        );
        for (i, p) in app.posts.iter().enumerate().skip(1) {
            assert_eq!(p.comments.len(), others[i], "post {i}");
        }

        // Search opens from the post view and Esc puts back the new selection.
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "maren");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::PostDetail { comment: 3 });
    }

    /// Everything deleting a comment must leave alone: every post's
    /// reactions and like state, and `post_sel`.
    fn untouched(app: &App) -> (Vec<(u32, bool)>, usize) {
        let posts = app.posts.iter().map(|p| (p.reactions, p.liked)).collect();
        (posts, app.post_sel)
    }

    fn bodies(app: &App, post: usize) -> Vec<String> {
        app.posts[post]
            .comments
            .iter()
            .map(|c| c.body.clone())
            .collect()
    }

    #[test]
    fn x_deletes_your_comment_and_keeps_the_index() {
        let mut app = maren_post_view();
        let seed = bodies(&app, 0);
        let others: Vec<usize> = app.posts.iter().map(|p| p.comments.len()).collect();
        for text in ["first", "second", "third"] {
            press(&mut app, KeyCode::Char('c'));
            type_str(&mut app, text);
            press(&mut app, KeyCode::Enter);
        }
        let before = untouched(&app);

        // In the middle: the next comment slides under the cursor.
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 4 });
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.status, Some("Comment deleted"));
        assert_eq!(app.mode, Mode::PostDetail { comment: 4 });
        let mut want = seed.clone();
        want.extend(["first".to_owned(), "third".to_owned()]);
        assert_eq!(bodies(&app, 0), want);

        // At the end: the cursor moves up.
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.status, Some("Comment deleted"));
        assert_eq!(app.mode, Mode::PostDetail { comment: 3 });
        want.pop();
        assert_eq!(bodies(&app, 0), want);
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        assert_eq!(bodies(&app, 0), seed);

        // Now on a seed comment: nothing, not even a toast.
        press(&mut app, KeyCode::Char('x'));
        assert!(app.status.is_none());
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        assert_eq!(bodies(&app, 0), seed);

        assert_eq!(untouched(&app), before);
        for (i, p) in app.posts.iter().enumerate().skip(1) {
            assert_eq!(p.comments.len(), others[i], "post {i}");
        }
    }

    #[test]
    fn x_deletes_the_only_comment() {
        let mut app = App::new();
        for _ in 0..7 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.posts[7].author, "Lucia Brandt");
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "Floor three thanks you");
        press(&mut app, KeyCode::Enter);
        let before = untouched(&app);

        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.status, Some("Comment deleted"));
        assert!(app.posts[7].comments.is_empty());
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });
        assert_eq!(untouched(&app), before);

        // With no comments left, x does nothing.
        press(&mut app, KeyCode::Char('x'));
        assert!(app.status.is_none());
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });
    }

    #[test]
    fn x_leaves_seed_comments_and_empty_posts_alone() {
        let mut app = App::new();
        // The seed has no comment of yours.
        assert!(
            app.posts
                .iter()
                .flat_map(|p| &p.comments)
                .all(|c| c.author != app.profile.name)
        );
        // In the feed, x does nothing.
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.status.is_none());

        press(&mut app, KeyCode::Enter);
        for comment in 0..3 {
            assert_eq!(app.mode, Mode::PostDetail { comment });
            press(&mut app, KeyCode::Char('x'));
            assert!(app.status.is_none(), "comment {comment}");
            assert_eq!(app.posts[0].comments.len(), 3);
            assert_eq!(app.mode, Mode::PostDetail { comment });
            press(&mut app, KeyCode::Char('j'));
        }

        // A post with no comments.
        let mut app = App::new();
        for _ in 0..7 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Enter);
        let before = untouched(&app);
        press(&mut app, KeyCode::Char('x'));
        assert!(app.status.is_none());
        assert!(app.posts[7].comments.is_empty());
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });
        assert_eq!(untouched(&app), before);
    }

    #[test]
    fn x_is_typed_in_the_comment_box() {
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "mine");
        press(&mut app, KeyCode::Enter);
        // Your own comment is selected, and x still only types.
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "x-ray");
        assert_eq!(app.comment_draft, "x-ray");
        assert_eq!(app.mode, Mode::CommentBox { comment: 3 });
        assert_eq!(app.posts[0].comments.len(), 4);
        assert!(app.status.is_none());
    }

    #[test]
    fn empty_and_blank_comments_are_rejected() {
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Write something first"));
        assert_eq!(app.mode, Mode::CommentBox { comment: 0 });

        type_str(&mut app, "   ");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Write something first"));
        assert_eq!(app.mode, Mode::CommentBox { comment: 0 }, "keeps focus");
        assert_eq!(app.comment_draft, "   ", "and the draft");
        assert_eq!(app.posts[0].comments.len(), 3);

        // The toast clears on the next key, which is typed.
        type_str(&mut app, "ok");
        assert!(app.status.is_none());
        assert_eq!(app.comment_draft, "   ok");
    }

    #[test]
    fn a_comment_survives_leaving_the_post_and_isnt_searched() {
        let mut app = App::new();
        for _ in 0..7 {
            press(&mut app, KeyCode::Char('j'));
        }
        assert_eq!(app.posts[7].author, "Lucia Brandt");
        assert!(app.posts[7].comments.is_empty());
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "zebra crossing");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });

        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('1'));
        assert_eq!(
            (app.tab, app.mode, app.post_sel),
            (Tab::Home, Mode::Browse, 7)
        );
        press(&mut app, KeyCode::Enter);
        let comments = &app.posts[7].comments;
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].body, "zebra crossing");

        // Comments aren't searched.
        assert!(app.search_hits("zebra").is_empty());
    }

    #[test]
    fn like_toggles_reactions() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        let before = app.posts[1].reactions;

        press(&mut app, KeyCode::Char('l'));
        assert!(app.posts[1].liked);
        assert_eq!(app.posts[1].reactions, before + 1);

        press(&mut app, KeyCode::Char('l'));
        assert!(!app.posts[1].liked);
        assert_eq!(app.posts[1].reactions, before);
    }

    #[test]
    fn post_detail_opens_and_esc_keeps_selection() {
        let mut app = App::new();
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });

        // j/k walk the comments, not the feed, and stop at the last one.
        let comments = app.posts[2].comments.len();
        for _ in 0..10 {
            press(&mut app, KeyCode::Char('j'));
        }
        assert_eq!(
            app.mode,
            Mode::PostDetail {
                comment: comments.saturating_sub(1)
            }
        );
        assert_eq!(app.post_sel, 2);

        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Browse);
        assert_eq!(app.post_sel, 2, "Esc returns to the same post");
    }

    #[test]
    fn composer_swallows_keys() {
        let mut app = App::new();
        let reactions = app.posts[0].reactions;
        press(&mut app, KeyCode::Char('p'));
        assert_eq!(app.mode, Mode::Composer);

        type_str(&mut app, "q1jl axcse");
        assert_eq!(app.draft, "q1jl axcse");
        assert!(!app.should_quit, "q is typed, not quit");
        assert_eq!(app.tab, Tab::Home, "1 is typed, not a tab switch");
        assert_eq!(app.post_sel, 0, "j is typed, not a move");
        assert_eq!(app.posts[0].reactions, reactions, "l is typed, not a like");
        assert!(!app.posts[0].liked);

        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.draft, "q1jl axcs");

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.tab, Tab::Home, "Tab doesn't switch tabs while typing");
        assert_eq!(app.mode, Mode::Composer);
    }

    #[test]
    fn composer_esc_discards_draft() {
        let mut app = App::new();
        let n = app.posts.len();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "half a thought");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.draft.is_empty());
        assert_eq!(app.posts.len(), n);

        press(&mut app, KeyCode::Char('p'));
        assert!(app.draft.is_empty(), "reopening starts from scratch");
    }

    #[test]
    fn composer_rejects_empty_and_whitespace_posts() {
        let mut app = App::new();
        let n = app.posts.len();
        press(&mut app, KeyCode::Char('p'));

        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Write something first"));
        assert_eq!(app.mode, Mode::Composer);

        type_str(&mut app, "   ");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Write something first"));
        assert_eq!(app.mode, Mode::Composer, "composer stays open");
        assert_eq!(app.posts.len(), n);
    }

    #[test]
    fn publishing_puts_post_first_and_selects_it() {
        let mut app = App::new();
        let n = app.posts.len();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "Hello WorkedIn");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.posts.len(), n + 1);
        assert_eq!(app.posts[0].body, "Hello WorkedIn");
        assert_eq!(app.posts[0].author, app.profile.name);
        assert_eq!(app.posts[0].hours_ago, 0);
        assert_eq!(app.post_sel, 0);
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.draft.is_empty());
        assert_eq!(app.status, Some("Post published"));
    }

    #[test]
    fn accept_and_ignore_invitations() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        let connections = app.profile.connections;
        assert_eq!(app.badge(Tab::Network), 3);

        let first = app.invitations[0].name;
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(app.status, Some("Invitation accepted"));
        assert_eq!(app.profile.connections, connections + 1);
        assert_eq!(app.badge(Tab::Network), 2);
        assert!(app.invitations.iter().all(|p| p.name != first));

        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.status, Some("Invitation ignored"));
        assert_eq!(
            app.profile.connections,
            connections + 1,
            "ignore doesn't connect"
        );
        assert_eq!(app.badge(Tab::Network), 1);
    }

    #[test]
    fn removal_keeps_selection_in_bounds() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.network_item(), Some(NetworkItem::Invitation(2)));
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(
            app.network_item(),
            Some(NetworkItem::Suggestion(0)),
            "the next row slides under the cursor"
        );

        // With no suggestions, removing the last row moves the cursor up.
        let mut app = App::new();
        app.suggestions.clear();
        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(app.network_sel, 1);
        assert_eq!(app.network_item(), Some(NetworkItem::Invitation(1)));
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('a'));
        assert!(app.invitations.is_empty());
        assert_eq!(app.network_sel, 0);
        assert_eq!(app.network_item(), None);
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.network_sel, 0, "an empty list doesn't move");
    }

    #[test]
    fn network_keys_only_act_on_their_section() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('c'));
        assert!(app.status.is_none(), "c on an invitation does nothing");
        assert!(app.suggestions.iter().all(|s| !s.pending));

        for _ in 0..3 {
            press(&mut app, KeyCode::Char('j'));
        }
        assert_eq!(app.network_item(), Some(NetworkItem::Suggestion(0)));
        let connections = app.profile.connections;
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.invitations.len(), 3);
        assert_eq!(app.suggestions.len(), 4);
        assert_eq!(app.profile.connections, connections);
        assert!(app.status.is_none(), "a/x on a suggestion do nothing");
    }

    #[test]
    fn connect_toggles_pending() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        for _ in 0..4 {
            press(&mut app, KeyCode::Char('j'));
        }
        assert_eq!(app.network_item(), Some(NetworkItem::Suggestion(1)));
        press(&mut app, KeyCode::Char('c'));
        assert!(app.suggestions[1].pending);
        assert_eq!(app.status, Some("Invitation sent"));
        press(&mut app, KeyCode::Char('c'));
        assert!(!app.suggestions[1].pending);
        assert_eq!(app.status, Some("Invitation withdrawn"));
    }

    #[test]
    fn easy_apply_twice_is_a_no_op() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('e'));
        assert!(app.jobs[1].applied);
        assert_eq!(app.status, Some("Application sent"));

        let before: Vec<(bool, bool)> = app.jobs.iter().map(|j| (j.saved, j.applied)).collect();
        press(&mut app, KeyCode::Char('e'));
        assert_eq!(app.status, Some("Already applied"));
        let after: Vec<(bool, bool)> = app.jobs.iter().map(|j| (j.saved, j.applied)).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn saving_a_job_updates_the_count() {
        let mut app = App::new();
        let saved = app.saved_jobs();
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('s'));
        assert!(app.jobs[1].saved);
        assert_eq!(app.saved_jobs(), saved + 1);
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.saved_jobs(), saved);
    }

    #[test]
    fn opening_a_conversation_marks_it_read() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        assert_eq!(app.badge(Tab::Messaging), 2);
        assert!(app.conversations[0].unread);

        press(&mut app, KeyCode::Enter);
        assert!(!app.conversations[0].unread);
        assert_eq!(app.badge(Tab::Messaging), 1);
        assert_eq!(app.open_chat, Some(0));
        assert_eq!(app.mode, Mode::Reply);
    }

    #[test]
    fn replies_append_and_swallow_keys() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        let n = app.conversations[1].messages.len();

        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "  ");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.conversations[1].messages.len(),
            n,
            "empty reply does nothing"
        );

        type_str(&mut app, "Thursday 2pm works; see you q5");
        assert_eq!(
            app.tab,
            Tab::Messaging,
            "digits are typed, not tab switches"
        );
        assert!(!app.should_quit);
        press(&mut app, KeyCode::Enter);
        let chat = &app.conversations[1];
        assert_eq!(chat.messages.len(), n + 1);
        let last = chat.messages.last().expect("just appended");
        assert!(last.from_me);
        assert_eq!(last.body, "Thursday 2pm works; see you q5");
        assert!(app.reply.is_empty());
        assert_eq!(app.mode, Mode::Reply, "the reply box keeps focus");

        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Browse);
        press(&mut app, KeyCode::Char('1'));
        assert_eq!(app.tab, Tab::Home);
    }

    #[test]
    fn opening_notifications_clears_badge() {
        let mut app = App::new();
        assert_eq!(app.badge(Tab::Notifications), 3);
        press(&mut app, KeyCode::Char('5'));
        assert_eq!(app.badge(Tab::Notifications), 0);

        let mut app = App::new();
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.tab, Tab::Notifications);
        assert_eq!(
            app.badge(Tab::Notifications),
            0,
            "any way of opening it counts"
        );
    }

    #[test]
    fn notifications_stay_new_for_one_visit() {
        let mut app = App::new();
        let new = |app: &App| -> Vec<bool> { app.notifications.iter().map(|n| n.is_new).collect() };
        let unread: Vec<bool> = app.notifications.iter().map(|n| n.unread).collect();
        assert_eq!(unread.iter().filter(|&&u| u).count(), 3);
        assert!(
            new(&app).iter().all(|&n| !n),
            "nothing is new before a visit"
        );

        press(&mut app, KeyCode::Char('5'));
        assert_eq!(app.badge(Tab::Notifications), 0, "the badge clears at once");
        assert_eq!(new(&app), unread, "this visit marks the unread ones as new");

        // Moving around, or pressing 5 again, is still the same visit.
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('5'));
        assert_eq!(new(&app), unread);

        press(&mut app, KeyCode::Char('1'));
        assert!(new(&app).iter().all(|&n| !n), "leaving clears the markers");
        press(&mut app, KeyCode::Char('5'));
        assert!(
            new(&app).iter().all(|&n| !n),
            "nothing is new on the next visit"
        );
        assert_eq!(app.badge(Tab::Notifications), 0);

        // Leaving with Tab (which wraps to Home) counts too.
        let mut app = App::new();
        press(&mut app, KeyCode::BackTab);
        assert_eq!(new(&app), unread);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.tab, Tab::Home);
        assert!(new(&app).iter().all(|&n| !n));
    }

    #[test]
    fn tab_and_backtab_wrap() {
        let mut app = App::new();
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.tab, Tab::Notifications);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.tab, Tab::Home);
        for want in [
            Tab::Network,
            Tab::Jobs,
            Tab::Messaging,
            Tab::Notifications,
            Tab::Home,
        ] {
            press(&mut app, KeyCode::Tab);
            assert_eq!(app.tab, want);
        }
    }

    #[test]
    fn switching_tabs_closes_post_detail() {
        let mut app = App::new();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('3'));
        assert_eq!(app.mode, Mode::Browse);
        press(&mut app, KeyCode::Char('1'));
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn q_quits_but_esc_does_not() {
        let mut app = App::new();
        press(&mut app, KeyCode::Esc);
        assert!(!app.should_quit);
        press(&mut app, KeyCode::Char('q'));
        assert!(app.should_quit);
    }

    #[test]
    fn toast_clears_on_next_key() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('e'));
        assert!(app.status.is_some());
        press(&mut app, KeyCode::Char('j'));
        assert!(app.status.is_none());
    }

    #[test]
    fn slash_opens_search_everywhere_and_esc_restores_everything() {
        // Every tab in Browse mode, each with a selection moved off 0.
        for tab in ['1', '2', '3', '4', '5'] {
            let mut app = App::new();
            for k in ['1', '2', '3', '4', '5'] {
                press(&mut app, KeyCode::Char(k));
                press(&mut app, KeyCode::Char('j'));
            }
            press(&mut app, KeyCode::Char('4'));
            press(&mut app, KeyCode::Enter);
            press(&mut app, KeyCode::Esc);
            press(&mut app, KeyCode::Char(tab));
            let before = snapshot(&app);

            press(&mut app, KeyCode::Char('/'));
            assert_eq!(query(&app), Some(""), "tab {tab}");
            type_str(&mut app, "ana");
            press(&mut app, KeyCode::Down);
            assert_eq!(app.tab, before.0, "the tab stays underneath");
            press(&mut app, KeyCode::Esc);
            assert!(app.search.is_none(), "tab {tab}");
            assert_eq!(snapshot(&app), before, "tab {tab}");

            // Reopening starts from an empty query.
            press(&mut app, KeyCode::Char('/'));
            assert_eq!(query(&app), Some(""));
        }

        // The post view, with its selected comment.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 1 });
        let before = snapshot(&app);
        press(&mut app, KeyCode::Char('/'));
        assert!(app.search.is_some());
        type_str(&mut app, "hana");
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Esc);
        assert!(app.search.is_none());
        assert_eq!(snapshot(&app), before);
        assert_eq!(app.mode, Mode::PostDetail { comment: 1 });
    }

    #[test]
    fn slash_is_typed_in_the_composer_and_reply_box() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "a/b");
        assert_eq!(app.draft, "a/b");
        assert!(app.search.is_none());
        assert_eq!(app.mode, Mode::Composer);

        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "and/or");
        assert_eq!(app.reply, "and/or");
        assert!(app.search.is_none());
        assert_eq!(app.mode, Mode::Reply);
    }

    #[test]
    fn search_box_types_keys_instead_of_acting_on_them() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        let before = snapshot(&app);
        let connections = app.profile.connections;
        press(&mut app, KeyCode::Char('/'));

        type_str(&mut app, "q1jlpaxcse5/");
        assert_eq!(query(&app), Some("q1jlpaxcse5/"));
        assert!(!app.should_quit, "q is typed, not quit");
        assert_eq!(snapshot(&app), before, "1, 5 and j are typed");
        assert_eq!(app.invitations.len(), 3, "a and x are typed");
        assert_eq!(app.profile.connections, connections);
        assert!(app.suggestions.iter().all(|s| !s.pending), "c is typed");
        assert!(app.status.is_none());

        // Shift is fine: capitals and symbols are typed.
        app.on_key(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT));
        assert_eq!(query(&app), Some("q1jlpaxcse5/Q"));

        // Tab, BackTab and Ctrl- or Alt-modified keys do nothing.
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::BackTab);
        for (code, modifiers) in [
            (KeyCode::Char('a'), KeyModifiers::CONTROL),
            (KeyCode::Char('x'), KeyModifiers::ALT),
            (KeyCode::Backspace, KeyModifiers::CONTROL),
            (KeyCode::Enter, KeyModifiers::ALT),
            (KeyCode::Esc, KeyModifiers::CONTROL),
            (KeyCode::Down, KeyModifiers::ALT),
        ] {
            app.on_key(KeyEvent::new(code, modifiers));
        }
        assert_eq!(query(&app), Some("q1jlpaxcse5/Q"));
        assert_eq!(snapshot(&app), before);
        assert!(app.status.is_none());

        // Backspace deletes, and on an empty query keeps search open.
        press(&mut app, KeyCode::Backspace);
        assert_eq!(query(&app), Some("q1jlpaxcse5/"));
        for _ in 0..20 {
            press(&mut app, KeyCode::Backspace);
        }
        assert_eq!(query(&app), Some(""), "still open");

        // Ctrl-C still quits.
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn bright_finds_people_jobs_and_posts() {
        let mut app = App::new();
        let want = [
            ("invitation", "Baptiste Moreau"),
            ("suggestion", "Mateo Achebe"),
            ("job", "Developer Advocate at Brightkettle"),
            ("post", "Tobias Ferreira-Lund"),
            ("post", "Hana Sorensen"),
        ]
        .map(|(kind, name)| (kind, name.to_owned()));
        assert_eq!(named_hits(&app, "BRIGHT"), want);
        assert_eq!(named_hits(&app, "brightkettle"), want);
        assert_eq!(named_hits(&app, "  bRiGhT "), want, "trimmed, any case");

        // Through the keys, too.
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "BRIGHT");
        assert_eq!(named_hits(&app, query(&app).expect("open")), want);

        // Other fields: a headline, a body, a job location.
        assert_eq!(named_hits(&app, "recruiter")[0].1, "Ines Okonkwo");
        assert!(named_hits(&app, "CI queue").contains(&("post", "Maren Holt".to_owned())));
        assert!(!named_hits(&app, "lisbon").is_empty());
        // Comments, messages, notifications and news aren't searched.
        assert!(named_hits(&app, "Number 5 is the real secret").is_empty());
        assert!(named_hits(&app, "standing desks").is_empty());
        // An empty or blank query matches nothing.
        assert!(app.search_hits("").is_empty());
        assert!(app.search_hits("   ").is_empty());
    }

    #[test]
    fn results_follow_the_current_state() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.invitations[1].name, "Baptiste Moreau");
        press(&mut app, KeyCode::Char('a'));
        let people: Vec<_> = named_hits(&app, "bright")
            .into_iter()
            .filter(|(kind, _)| kind.ends_with("ion"))
            .collect();
        assert_eq!(people, [("suggestion", "Mateo Achebe".to_owned())]);

        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "Rust meetup at Brightkettle");
        press(&mut app, KeyCode::Enter);
        let hits = app.search_hits("bright");
        let first_post = hits
            .iter()
            .find(|h| matches!(h, SearchHit::Post(_)))
            .expect("posts match");
        assert_eq!(*first_post, SearchHit::Post(0));
        assert_eq!(app.posts[0].body, "Rust meetup at Brightkettle");

        // Connecting to a suggestion is seen too (the overlay draws it as
        // Pending; see the ui tests).
        press(&mut app, KeyCode::Char('2'));
        for _ in 0..10 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.search_hits("mateo"), [SearchHit::Suggestion(3)]);
        assert!(app.suggestions[3].pending);
    }

    #[test]
    fn enter_jumps_to_the_selected_result() {
        let open = |app: &mut App, down: usize| {
            press(app, KeyCode::Char('/'));
            type_str(app, "bright");
            for _ in 0..down {
                press(app, KeyCode::Down);
            }
            press(app, KeyCode::Enter);
            assert!(app.search.is_none(), "Enter closes search");
            assert!(app.status.is_none());
        };

        // Invitation 1.
        let mut app = App::new();
        open(&mut app, 0);
        assert_eq!((app.tab, app.mode), (Tab::Network, Mode::Browse));
        assert_eq!(app.network_item(), Some(NetworkItem::Invitation(1)));

        // Suggestion 3 sits after the three invitations.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        open(&mut app, 1);
        assert_eq!(app.tab, Tab::Network);
        assert_eq!(app.network_sel, 6);
        assert_eq!(app.network_item(), Some(NetworkItem::Suggestion(3)));

        // The job.
        let mut app = App::new();
        open(&mut app, 2);
        assert_eq!(app.tab, Tab::Jobs);
        assert_eq!(app.jobs[app.job_sel].company, "Brightkettle");

        // A post, from the post view of another one: lands in the feed.
        let mut app = App::new();
        for _ in 0..4 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('j'));
        open(&mut app, 4);
        assert_eq!((app.tab, app.mode), (Tab::Home, Mode::Browse));
        assert_eq!(app.posts[app.post_sel].author, "Hana Sorensen");

        // And from another tab.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        open(&mut app, 3);
        assert_eq!((app.tab, app.mode), (Tab::Home, Mode::Browse));
        assert_eq!(app.post_sel, 1);

        // Reopening starts from an empty query.
        press(&mut app, KeyCode::Char('/'));
        assert_eq!(query(&app), Some(""));
    }

    #[test]
    fn enter_without_results_toasts_and_stays_open() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("No results"));
        assert_eq!(query(&app), Some(""));

        type_str(&mut app, "zqzq");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("No results"));
        assert_eq!(query(&app), Some("zqzq"), "the query is kept");
        assert_eq!(app.tab, Tab::Jobs);

        type_str(&mut app, "x");
        assert!(app.status.is_none(), "the toast clears on the next key");
    }

    #[test]
    fn selection_clamps_and_resets_when_the_query_changes() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        // Moving with no results stays at 0.
        press(&mut app, KeyCode::Down);
        assert_eq!(selected_hit(&app), 0);

        type_str(&mut app, "bright");
        press(&mut app, KeyCode::Up);
        assert_eq!(selected_hit(&app), 0, "clamps at the top");
        for _ in 0..10 {
            press(&mut app, KeyCode::Down);
        }
        assert_eq!(selected_hit(&app), 4, "clamps at the bottom");
        press(&mut app, KeyCode::Up);
        assert_eq!(selected_hit(&app), 3);

        press(&mut app, KeyCode::Char('k'));
        assert_eq!(selected_hit(&app), 0, "typing resets the selection");
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Backspace);
        assert_eq!(selected_hit(&app), 0, "so does deleting");
        assert_eq!(query(&app), Some("bright"));
    }

    #[test]
    fn jumping_from_notifications_ends_the_visit() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('5'));
        let new = |app: &App| app.notifications.iter().filter(|n| n.is_new).count();
        assert_eq!(new(&app), 3);

        // Esc is still the same visit.
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "bright");
        press(&mut app, KeyCode::Esc);
        assert_eq!(new(&app), 3);

        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "bright");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.tab, Tab::Network);
        assert_eq!(new(&app), 0, "leaving by a jump clears the markers");
        assert_eq!(app.badge(Tab::Notifications), 0);
    }
}
