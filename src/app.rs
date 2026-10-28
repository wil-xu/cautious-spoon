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

/// A one-line text box over a list: what search and the Send picker share.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FilterBox {
    pub query: String,
    /// Index into the list the query gives.
    pub selected: usize,
}

/// The keys a `FilterBox` leaves to its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoxKey {
    Enter,
    Esc,
}

impl FilterBox {
    /// Typing, `Backspace` and `↑`/`↓` over a list of `len` items. Any change
    /// to the query selects the first item again, and the selection stops at
    /// both ends. Ctrl- and Alt-modified keys, `Tab`, `BackTab` and every other
    /// key do nothing. `Enter` and `Esc` are returned for the owner to act on.
    fn on_key(&mut self, key: KeyEvent, len: usize) -> Option<BoxKey> {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        match key.code {
            KeyCode::Enter => return Some(BoxKey::Enter),
            KeyCode::Esc => return Some(BoxKey::Esc),
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(len.saturating_sub(1)),
            KeyCode::Backspace => {
                if self.query.pop().is_some() {
                    self.selected = 0;
                }
            }
            _ => {
                if let Some(ch) = typed(key) {
                    self.query.push(ch);
                    self.selected = 0;
                }
            }
        }
        None
    }
}

/// The search overlay, open over whatever tab and mode it was opened from.
/// Those stay untouched underneath, so closing it puts everything back.
/// `selected` indexes `App::search_hits`.
pub(crate) type Search = FilterBox;

/// One search result, pointing into the list it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchHit {
    Invitation(usize),
    Suggestion(usize),
    Job(usize),
    Post(usize),
    /// Comment `comment` of post `post`.
    Comment {
        post: usize,
        comment: usize,
    },
    /// Conversation `chat`, showing its message `message`: the newest one
    /// whose body matches, or the newest one when only the name matches.
    /// `None` only for a conversation with no messages.
    Conversation {
        chat: usize,
        message: Option<usize>,
    },
}

/// What a search query matches against: trimmed and lowercased. The same
/// lowercasing goes on every searched field, so case never matters.
pub(crate) fn search_needle(query: &str) -> String {
    query.trim().to_lowercase()
}

/// The Send picker, open over the Home feed or post view it was opened from.
/// Like search, it leaves `tab`, `mode` and every selection untouched
/// underneath, so closing it puts everything back. `query` is the filter, and
/// `selected` indexes `App::send_recipients`.
pub(crate) type SendPicker = FilterBox;

/// A post's first line: what the Send picker shows and what sending shares.
pub(crate) fn first_line(body: &str) -> &str {
    body.lines().next().unwrap_or("")
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
    /// Lines of the Jobs details card scrolled off the top, at most
    /// `job_scroll_max`. Back to 0 whenever `job_sel` changes.
    pub job_scroll: usize,
    /// How far the details card can scroll, set by `fit_job_details`.
    pub job_scroll_max: usize,
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
    /// `Some` while the Send picker is open. Takes every key but Ctrl-C.
    pub send: Option<SendPicker>,
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
            job_scroll: 0,
            job_scroll_max: 0,
            chat_sel: 0,
            notif_sel: 0,
            open_chat: None,
            draft: String::new(),
            reply: String::new(),
            comment_draft: String::new(),
            search: None,
            send: None,
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

    /// Tells the app how far the selected job's details can scroll on the
    /// current screen: the lines that don't fit in the card, as
    /// `ui::job_scroll_max` measures them. Only the event loop knows the screen
    /// size, so it calls this before every frame. A screen that grew pulls the
    /// scroll back, so the next `K` moves the text at once.
    pub(crate) fn fit_job_details(&mut self, max: usize) {
        self.job_scroll_max = max;
        self.job_scroll = self.job_scroll.min(max);
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
    /// (invitations, then suggestions), then jobs, then posts in feed order,
    /// then comments (by post in feed order, then as the post view lists
    /// them), then conversations in Messaging list order. A comment matches
    /// by its body only, and a conversation by its name or any message body,
    /// once however many messages match. A query that is empty once trimmed
    /// matches nothing.
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
        for (post, p) in self.posts.iter().enumerate() {
            for (comment, c) in p.comments.iter().enumerate() {
                if found(&[&c.body]) {
                    hits.push(SearchHit::Comment { post, comment });
                }
            }
        }
        for (chat, c) in self.conversations.iter().enumerate() {
            let newest_match = c.messages.iter().rposition(|m| found(&[&m.body]));
            if newest_match.is_some() || found(&[c.name]) {
                let message = newest_match.or_else(|| c.messages.len().checked_sub(1));
                hits.push(SearchHit::Conversation { chat, message });
            }
        }
        hits
    }

    /// The conversations the Send picker offers for `filter`, as indexes into
    /// `conversations`, in Messaging list order. Only names are matched, the
    /// way search matches a field. A filter that is empty once trimmed lists
    /// every conversation.
    pub(crate) fn send_recipients(&self, filter: &str) -> Vec<usize> {
        let needle = search_needle(filter);
        self.conversations
            .iter()
            .enumerate()
            .filter(|(_, c)| c.name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect()
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
            (Tab::Jobs, _) => {
                let before = self.job_sel;
                step(&mut self.job_sel, self.jobs.len(), delta);
                if self.job_sel != before {
                    self.job_scroll = 0;
                }
            }
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
        if self.send.is_some() {
            self.on_send_key(key);
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
        let hits = match &self.search {
            Some(search) => self.search_hits(&search.query),
            None => return,
        };
        let Some(search) = self.search.as_mut() else {
            return;
        };
        match search.on_key(key, hits.len()) {
            Some(BoxKey::Esc) => self.search = None,
            Some(BoxKey::Enter) => match hits.get(search.selected) {
                Some(&hit) => self.open_hit(hit),
                None => self.status = Some("No results"),
            },
            None => {}
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
                self.job_scroll = 0;
            }
            SearchHit::Post(i) => {
                self.set_tab(Tab::Home);
                self.post_sel = i;
            }
            SearchHit::Comment { post, comment } => {
                self.set_tab(Tab::Home);
                self.post_sel = post;
                self.mode = Mode::PostDetail { comment };
            }
            SearchHit::Conversation { chat, .. } => {
                self.set_tab(Tab::Messaging);
                self.chat_sel = chat;
                self.open_conversation();
            }
        }
    }

    fn on_home_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('s') if self.selected_post().is_some() => {
                self.send = Some(SendPicker::default());
            }
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

    /// The Send picker is a text box, like search: Ctrl- and Alt-modified keys
    /// do nothing at all.
    fn on_send_key(&mut self, key: KeyEvent) {
        let recipients = match &self.send {
            Some(picker) => self.send_recipients(&picker.query),
            None => return,
        };
        let Some(picker) = self.send.as_mut() else {
            return;
        };
        match picker.on_key(key, recipients.len()) {
            Some(BoxKey::Esc) => self.send = None,
            Some(BoxKey::Enter) => match recipients.get(picker.selected) {
                Some(&chat) => self.send_post(chat),
                None => self.status = Some("No matches"),
            },
            None => {}
        }
    }

    /// Closes the picker and appends the selected post to conversation `chat`
    /// as a message from you. Nothing else changes: not `unread`, the order of
    /// the conversations, the Messaging selection, the open thread or the
    /// reply draft.
    fn send_post(&mut self, chat: usize) {
        self.send = None;
        let Some(post) = self.posts.get(self.post_sel) else {
            return;
        };
        let body = format!(
            "Shared a post by {}: {}",
            post.author,
            first_line(&post.body)
        );
        let Some(chat) = self.conversations.get_mut(chat) else {
            return;
        };
        chat.messages.push(Message {
            from_me: true,
            body,
            hours_ago: 0,
        });
        self.status = Some("Post sent");
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
            KeyCode::Char('J') => self.job_scroll = (self.job_scroll + 1).min(self.job_scroll_max),
            KeyCode::Char('K') => self.job_scroll = self.job_scroll.saturating_sub(1),
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
        if code == KeyCode::Enter {
            self.open_conversation();
        }
    }

    /// Opens the selected conversation on the right of Messaging: marks it
    /// read, clears the reply draft and focuses the reply box.
    fn open_conversation(&mut self) {
        if let Some(chat) = self.conversations.get_mut(self.chat_sel) {
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
                SearchHit::Comment { post, comment } => (
                    "comment",
                    format!(
                        "{} on {}",
                        app.posts[post].comments[comment].author, app.posts[post].author
                    ),
                ),
                SearchHit::Conversation { chat, .. } => {
                    ("conversation", app.conversations[chat].name.to_owned())
                }
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
    fn a_comment_survives_leaving_the_post_and_is_searched() {
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

        // The comment is a result, and doesn't make its post one.
        assert_eq!(
            app.search_hits("zebra"),
            [SearchHit::Comment {
                post: 7,
                comment: 0
            }]
        );
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
        // Comments are searched; notifications and news aren't.
        assert_eq!(
            named_hits(&app, "Number 5 is the real secret"),
            [(
                "comment",
                "Tobias Ferreira-Lund on Casey Lindqvist".to_owned()
            )]
        );
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

    fn filter(app: &App) -> Option<&str> {
        app.send.as_ref().map(|p| p.query.as_str())
    }

    fn selected_recipient(app: &App) -> usize {
        app.send.as_ref().expect("the picker is open").selected
    }

    fn recipient_names(app: &App, filter: &str) -> Vec<&'static str> {
        app.send_recipients(filter)
            .into_iter()
            .map(|i| app.conversations[i].name)
            .collect()
    }

    /// Every conversation's messages, to compare before and after sending.
    fn messages(app: &App) -> Vec<Vec<(bool, String, u32)>> {
        app.conversations
            .iter()
            .map(|c| {
                c.messages
                    .iter()
                    .map(|m| (m.from_me, m.body.clone(), m.hours_ago))
                    .collect()
            })
            .collect()
    }

    /// Everything sending must leave alone besides the recipient's messages:
    /// the conversations' order and `unread` flags, the Messaging badge, the
    /// reply draft, and every post's reactions, like state and comments.
    type Unsent = (
        Vec<(&'static str, bool)>,
        usize,
        String,
        Vec<(u32, bool, Vec<String>)>,
    );

    fn unsent(app: &App) -> Unsent {
        (
            app.conversations
                .iter()
                .map(|c| (c.name, c.unread))
                .collect(),
            app.badge(Tab::Messaging),
            app.reply.clone(),
            app.posts
                .iter()
                .map(|p| {
                    let comments = p.comments.iter().map(|c| c.body.clone()).collect();
                    (p.reactions, p.liked, comments)
                })
                .collect(),
        )
    }

    const MAREN_SHARED: &str = "Shared a post by Maren Holt: I'm happy to share that our \
                                team cut the CI queue from 40 minutes to 6.";

    #[test]
    fn s_opens_the_send_picker_from_the_feed_and_the_post_view() {
        // The feed, with the second post selected.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        let before = snapshot(&app);
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(filter(&app), Some(""));
        assert_eq!(selected_recipient(&app), 0);
        assert_eq!(snapshot(&app), before, "the feed stays underneath");
        assert!(app.status.is_none());
        type_str(&mut app, "ravi");
        press(&mut app, KeyCode::Esc);
        assert!(app.send.is_none());
        assert_eq!(snapshot(&app), before);
        assert_eq!((app.mode, app.post_sel), (Mode::Browse, 1));
        // Reopening starts from an empty filter.
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(filter(&app), Some(""));

        // The post view, with the third comment selected.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        let before = snapshot(&app);
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(filter(&app), Some(""));
        type_str(&mut app, "felix");
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Esc);
        assert!(app.send.is_none());
        assert_eq!(snapshot(&app), before);
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        assert_eq!(messages(&app), messages(&App::new()), "Esc sends nothing");

        // Search still opens from the post view, and Esc keeps the comment.
        press(&mut app, KeyCode::Char('/'));
        assert_eq!(query(&app), Some(""));
        assert!(app.send.is_none());
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
    }

    #[test]
    fn s_does_nothing_elsewhere_and_is_typed_in_text_boxes() {
        // Jobs: still save and unsave.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        let saved = app.jobs[0].saved;
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.jobs[0].saved, !saved);
        assert!(app.send.is_none());
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.jobs[0].saved, saved);
        assert!(app.send.is_none());

        // My Network, Messaging and Notifications: nothing at all.
        for tab in ['2', '4', '5'] {
            let mut app = App::new();
            press(&mut app, KeyCode::Char(tab));
            let before = snapshot(&app);
            let kept = unsent(&app);
            press(&mut app, KeyCode::Char('s'));
            assert!(app.send.is_none(), "tab {tab}");
            assert!(app.status.is_none(), "tab {tab}");
            assert_eq!(snapshot(&app), before, "tab {tab}");
            assert_eq!(unsent(&app), kept, "tab {tab}");
            assert_eq!(app.invitations.len(), 3);
            assert!(app.suggestions.iter().all(|s| !s.pending));
        }

        // Typed in the composer, the comment box, the reply box and search.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "s");
        assert_eq!((app.draft.as_str(), app.mode), ("s", Mode::Composer));
        assert!(app.send.is_none());

        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "s");
        assert_eq!(app.comment_draft, "s");
        assert_eq!(app.mode, Mode::CommentBox { comment: 0 });
        assert!(app.send.is_none());

        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "s");
        assert_eq!((app.reply.as_str(), app.mode), ("s", Mode::Reply));
        assert!(app.send.is_none());

        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "s");
        assert_eq!(query(&app), Some("s"));
        assert!(app.send.is_none(), "the picker can't open from search");
    }

    #[test]
    fn the_send_picker_types_keys_instead_of_acting_on_them() {
        // Your own comment is selected, so `x` could delete it.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "mine");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::PostDetail { comment: 3 });
        let before = snapshot(&app);
        let kept = unsent(&app);
        let sent = messages(&app);
        press(&mut app, KeyCode::Char('s'));

        type_str(&mut app, "q1jklcxps/");
        assert_eq!(filter(&app), Some("q1jklcxps/"));
        assert!(!app.should_quit, "q is typed, not quit");
        assert_eq!(snapshot(&app), before, "1, j and k are typed");
        assert_eq!(
            app.mode,
            Mode::PostDetail { comment: 3 },
            "c is typed, not the comment box"
        );
        assert_eq!(unsent(&app), kept, "l is typed, not a like; x not a delete");
        assert_eq!(app.posts[0].comments.len(), 4);
        assert!(app.draft.is_empty(), "p is typed, not the composer");
        assert!(app.search.is_none(), "/ is typed, not search");
        assert!(app.status.is_none());

        // Shift is fine.
        app.on_key(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT));
        assert_eq!(filter(&app), Some("q1jklcxps/Q"));

        // Tab, BackTab and Ctrl- or Alt-modified keys do nothing.
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::BackTab);
        for (code, modifiers) in [
            (KeyCode::Char('a'), KeyModifiers::CONTROL),
            (KeyCode::Char('l'), KeyModifiers::ALT),
            (KeyCode::Char('1'), KeyModifiers::ALT),
            (KeyCode::Backspace, KeyModifiers::CONTROL),
            (KeyCode::Backspace, KeyModifiers::ALT),
            (KeyCode::Enter, KeyModifiers::CONTROL),
            (KeyCode::Enter, KeyModifiers::ALT),
            (KeyCode::Esc, KeyModifiers::CONTROL),
            (KeyCode::Down, KeyModifiers::ALT),
        ] {
            app.on_key(KeyEvent::new(code, modifiers));
        }
        assert_eq!(filter(&app), Some("q1jklcxps/Q"));
        assert_eq!(snapshot(&app), before);
        assert_eq!(unsent(&app), kept);
        assert_eq!(messages(&app), sent, "nothing was sent");
        assert!(app.status.is_none());

        // Backspace deletes, and on an empty filter keeps the picker open.
        press(&mut app, KeyCode::Backspace);
        assert_eq!(filter(&app), Some("q1jklcxps/"));
        for _ in 0..20 {
            press(&mut app, KeyCode::Backspace);
        }
        assert_eq!(filter(&app), Some(""), "still open");
        assert_eq!(snapshot(&app), before);

        // In the feed too, p and l are typed.
        let mut app = App::new();
        let kept = unsent(&app);
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "pl");
        assert_eq!(filter(&app), Some("pl"));
        assert_eq!(app.mode, Mode::Browse, "not the composer");
        assert_eq!(unsent(&app), kept, "not a like");

        // Ctrl-C still quits.
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn the_send_filter_matches_names_only() {
        let mut app = App::new();
        let all = [
            "Maren Holt",
            "Ravi Tenorio",
            "Ines Okonkwo",
            "Felix Abernathy",
        ];
        assert_eq!(recipient_names(&app, ""), all);
        assert_eq!(recipient_names(&app, "   "), all, "blank lists everyone");
        assert_eq!(recipient_names(&app, "  MAREN "), ["Maren Holt"]);
        assert_eq!(
            recipient_names(&app, "o"),
            ["Maren Holt", "Ravi Tenorio", "Ines Okonkwo"]
        );
        // Headlines and messages don't match.
        assert!(recipient_names(&app, "Quillfeather").is_empty());
        assert!(recipient_names(&app, "recruiter").is_empty());
        assert!(recipient_names(&app, "coffee chat").is_empty());

        // Through the keys, too.
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "  MAREN ");
        let typed = filter(&app).expect("open").to_owned();
        assert_eq!(recipient_names(&app, &typed), ["Maren Holt"]);
    }

    #[test]
    fn send_selection_clamps_and_resets_when_the_filter_changes() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('s'));
        press(&mut app, KeyCode::Up);
        assert_eq!(selected_recipient(&app), 0, "clamps at the top");
        for _ in 0..10 {
            press(&mut app, KeyCode::Down);
        }
        assert_eq!(selected_recipient(&app), 3, "clamps at the bottom");
        press(&mut app, KeyCode::Up);
        assert_eq!(selected_recipient(&app), 2);

        type_str(&mut app, "o");
        assert_eq!(selected_recipient(&app), 0, "typing resets the selection");
        for _ in 0..10 {
            press(&mut app, KeyCode::Down);
        }
        assert_eq!(selected_recipient(&app), 2, "three matches");
        press(&mut app, KeyCode::Backspace);
        assert_eq!(selected_recipient(&app), 0, "so does deleting");
        assert_eq!(filter(&app), Some(""));

        // Moving with no matches stays at 0.
        type_str(&mut app, "zqzq");
        press(&mut app, KeyCode::Down);
        assert_eq!(selected_recipient(&app), 0);
    }

    #[test]
    fn sending_maren_holts_post_to_felix() {
        // Ines Okonkwo's thread open on Messaging, and half a reply.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('1'));
        app.reply = "half a reply".to_owned();
        assert_eq!((app.chat_sel, app.open_chat), (2, Some(2)));
        assert_eq!(app.badge(Tab::Messaging), 2);

        // Maren Holt's post view, with the second comment selected.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 1 });
        let before = snapshot(&app);
        let kept = unsent(&app);
        let sent = messages(&app);

        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "felix");
        assert_eq!(recipient_names(&app, "felix"), ["Felix Abernathy"]);
        press(&mut app, KeyCode::Enter);

        assert!(app.send.is_none(), "Enter closes the picker");
        assert_eq!(app.status, Some("Post sent"));
        assert_eq!(snapshot(&app), before, "back where you were");
        assert_eq!(app.mode, Mode::PostDetail { comment: 1 });
        assert_eq!(unsent(&app), kept);
        assert_eq!(app.badge(Tab::Messaging), 2);
        assert_eq!(app.reply, "half a reply");
        let after = messages(&app);
        assert_eq!(app.conversations[3].name, "Felix Abernathy");
        for (i, (was, now)) in sent.iter().zip(&after).enumerate() {
            if i == 3 {
                let mut want = was.clone();
                want.push((true, MAREN_SHARED.to_owned(), 0));
                assert_eq!(now, &want, "appended to Felix's conversation");
            } else {
                assert_eq!(now, was, "conversation {i}");
            }
        }

        // Sending it again sends a second message.
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "felix");
        press(&mut app, KeyCode::Enter);
        let felix = &messages(&app)[3];
        assert_eq!(felix.len(), sent[3].len() + 2);
        assert_eq!(felix[felix.len() - 2], felix[felix.len() - 1]);
        assert_eq!(app.mode, Mode::PostDetail { comment: 1 });

        // Felix's conversation now matches, once.
        assert_eq!(
            app.search_hits("Shared a post"),
            [SearchHit::Conversation {
                chat: 3,
                message: Some(4)
            }]
        );
    }

    #[test]
    fn sending_from_the_feed_returns_to_the_feed() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        let before = snapshot(&app);
        let post = &app.posts[2];
        let want = format!(
            "Shared a post by {}: {}",
            post.author,
            first_line(&post.body)
        );
        assert!(want.starts_with("Shared a post by Ines Okonkwo: We're hiring!"));

        press(&mut app, KeyCode::Char('s'));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Post sent"));
        assert_eq!(snapshot(&app), before);
        assert_eq!((app.mode, app.post_sel), (Mode::Browse, 2));
        let ravi = &app.conversations[1];
        assert_eq!(ravi.name, "Ravi Tenorio");
        let last = ravi.messages.last().expect("just sent");
        assert_eq!(
            (last.from_me, last.body.as_str(), last.hours_ago),
            (true, want.as_str(), 0)
        );
    }

    #[test]
    fn sending_to_an_unread_conversation_keeps_it_unread() {
        let mut app = App::new();
        assert!(app.conversations[0].unread);
        assert_eq!(app.badge(Tab::Messaging), 2);
        press(&mut app, KeyCode::Char('s'));
        // An empty filter lists everyone, Maren Holt first.
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Post sent"));
        let maren = &app.conversations[0];
        assert_eq!(maren.name, "Maren Holt");
        assert_eq!(maren.messages.last().expect("sent").body, MAREN_SHARED);
        assert!(maren.unread, "still unread");
        assert!(app.conversations[1].unread);
        assert_eq!(app.badge(Tab::Messaging), 2);
    }

    #[test]
    fn enter_without_matches_toasts_and_keeps_the_picker() {
        let mut app = App::new();
        let sent = messages(&app);
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "zqzq");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("No matches"));
        assert_eq!(filter(&app), Some("zqzq"), "open, with the filter");
        assert_eq!(messages(&app), sent);
        assert_eq!(app.mode, Mode::Browse);

        type_str(&mut app, "x");
        assert!(app.status.is_none(), "the toast clears on the next key");
        assert_eq!(filter(&app), Some("zqzqx"));
    }

    #[test]
    fn a_published_post_is_sent_under_your_name() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "  Shipping the Send action today  ");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.post_sel, 0);
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "ines");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Post sent"));
        let ines = &app.conversations[2];
        assert_eq!(
            ines.messages.last().expect("sent").body,
            "Shared a post by Jordan Vale: Shipping the Send action today"
        );
    }

    fn named(pairs: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
        pairs
            .iter()
            .map(|&(kind, name)| (kind, name.to_owned()))
            .collect()
    }

    #[test]
    fn comments_and_conversations_are_found_in_order() {
        let app = App::new();
        assert_eq!(
            named_hits(&app, "typo"),
            named(&[
                ("post", "Felix Abernathy"),
                ("comment", "Maren Holt on Felix Abernathy"),
                ("conversation", "Felix Abernathy"),
            ])
        );
        assert_eq!(
            app.search_hits("typo"),
            [
                SearchHit::Post(6),
                SearchHit::Comment {
                    post: 6,
                    comment: 0
                },
                SearchHit::Conversation {
                    chat: 3,
                    message: Some(2)
                },
            ]
        );
        // Maren Holt's post says "cached", so only the comment and the
        // message match.
        assert_eq!(
            named_hits(&app, "caching"),
            named(&[
                ("comment", "Hana Sorensen on Maren Holt"),
                ("conversation", "Maren Holt"),
            ])
        );
        // Felix wrote two comments, but a comment's author isn't searched.
        assert_eq!(
            named_hits(&app, "felix"),
            named(&[
                ("post", "Felix Abernathy"),
                ("conversation", "Felix Abernathy"),
            ])
        );
        // Ines Okonkwo has two matching messages and is one result.
        assert_eq!(
            named_hits(&app, "thanks"),
            named(&[
                ("post", "Wendell Achterberg"),
                ("conversation", "Ines Okonkwo"),
                ("conversation", "Felix Abernathy"),
            ])
        );
        assert_eq!(
            app.conversations[2]
                .messages
                .iter()
                .filter(|m| m.body.to_lowercase().contains("thanks"))
                .count(),
            2
        );
        // The message each conversation result shows, chosen here: the
        // newest matching one (Ines Okonkwo's is your 28h reply, not her
        // older message; Felix Abernathy's is his only match, not his newest
        // message), or the newest one when only the name matches.
        let conversations = |query: &str| -> Vec<(usize, Option<usize>)> {
            app.search_hits(query)
                .into_iter()
                .filter_map(|h| match h {
                    SearchHit::Conversation { chat, message } => Some((chat, message)),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(conversations("thanks"), [(2, Some(1)), (3, Some(0))]);
        assert_eq!(
            app.conversations[2].messages[1].body,
            "Thanks Ines, I'll keep it in mind."
        );
        assert!(app.conversations[2].messages[1].from_me);
        assert_eq!(
            app.conversations[3].messages[0].body,
            "Thanks for reviewing my first PR!"
        );
        assert_eq!(app.conversations[3].messages.len(), 3);
        assert_eq!(conversations("felix"), [(3, Some(2))]);
        assert_eq!(
            app.conversations[3].messages[2].body,
            "On to the next typo."
        );
        // `bright` is unchanged.
        assert_eq!(
            named_hits(&app, "bright"),
            named(&[
                ("invitation", "Baptiste Moreau"),
                ("suggestion", "Mateo Achebe"),
                ("job", "Developer Advocate at Brightkettle"),
                ("post", "Tobias Ferreira-Lund"),
                ("post", "Hana Sorensen"),
            ])
        );
        // Comments by post in feed order, then as the post view lists them.
        assert_eq!(
            app.search_hits("the")
                .into_iter()
                .filter_map(|h| match h {
                    SearchHit::Comment { post, comment } => Some((post, comment)),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [
                (0, 1),
                (0, 2),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 1),
                (6, 0),
                (8, 1)
            ]
        );
        // Trimmed and any case, as before.
        assert_eq!(named_hits(&app, "  TyPo "), named_hits(&app, "typo"));
    }

    #[test]
    fn authors_headlines_news_and_notifications_dont_match() {
        let app = App::new();
        let kinds = |query: &str| -> Vec<&'static str> {
            named_hits(&app, query)
                .into_iter()
                .map(|(k, _)| k)
                .collect()
        };
        // Noor Haddad commented on Ines Okonkwo's post: only the person.
        assert_eq!(kinds("noor haddad"), ["invitation"]);
        // Every comment headline and conversation headline says
        // Quillfeather, and no comment body or message does.
        assert!(
            app.posts
                .iter()
                .flat_map(|p| &p.comments)
                .any(|c| c.headline.contains("Quillfeather"))
        );
        let hits = kinds("quillfeather");
        assert!(!hits.is_empty());
        assert!(
            hits.iter().all(|&k| k != "comment" && k != "conversation"),
            "{hits:?}"
        );
        assert!(
            kinds("Technical Recruiter")
                .iter()
                .all(|&k| k != "conversation")
        );
        // News and notifications.
        assert!(app.search_hits("standing desks").is_empty());
        assert!(app.search_hits("impressions").is_empty());
        assert!(app.search_hits("").is_empty());
        assert!(app.search_hits("   ").is_empty());
    }

    /// Opens search, types `query`, moves down `down` results and presses
    /// Enter.
    fn jump(app: &mut App, query: &str, down: usize) {
        press(app, KeyCode::Char('/'));
        type_str(app, query);
        for _ in 0..down {
            press(app, KeyCode::Down);
        }
        press(app, KeyCode::Enter);
        assert!(app.search.is_none(), "Enter closes search");
    }

    #[test]
    fn enter_on_a_comment_opens_its_post_with_the_comment_selected() {
        // From the post view of another post, with another comment selected.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        jump(&mut app, "typo", 1);
        assert_eq!(app.tab, Tab::Home);
        assert_eq!(app.posts[app.post_sel].author, "Felix Abernathy");
        assert_eq!(app.post_sel, 6);
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });
        let post = app.selected_post().expect("a post");
        assert_eq!(post.comments[0].author, "Maren Holt");
        assert_eq!(
            post.comments[0].body,
            "Welcome aboard. The typo had it coming."
        );
        assert!(app.status.is_none());
        // Esc goes back to the feed, on Felix's post.
        press(&mut app, KeyCode::Esc);
        assert_eq!(
            (app.tab, app.mode, app.post_sel),
            (Tab::Home, Mode::Browse, 6)
        );

        // From another tab, a comment that isn't the first: j/k carry on
        // from it.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        jump(&mut app, "roadmap", 0);
        assert_eq!(app.tab, Tab::Home);
        assert_eq!(app.posts[app.post_sel].author, "Wendell Achterberg");
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 1 });
        assert_eq!(app.post_sel, 8);
    }

    #[test]
    fn enter_on_a_conversation_opens_it_like_messaging_does() {
        let mut app = App::new();
        app.reply = "a stale draft".to_owned();
        assert_eq!(app.badge(Tab::Messaging), 2);
        assert!(app.conversations[1].unread);
        assert_eq!(
            named_hits(&app, "agenda"),
            named(&[
                ("post", "Ravi Tenorio"),
                ("comment", "Felix Abernathy on Ravi Tenorio"),
                ("conversation", "Ravi Tenorio"),
            ])
        );
        jump(&mut app, "agenda", 2);
        assert_eq!(app.tab, Tab::Messaging);
        assert_eq!(app.chat_sel, 1);
        assert_eq!(app.open_chat, Some(1));
        assert!(!app.conversations[1].unread, "marked read");
        assert_eq!(app.badge(Tab::Messaging), 1);
        assert!(app.reply.is_empty());
        assert_eq!(app.mode, Mode::Reply);
        // The reply box has focus: keys are typed and Enter sends.
        type_str(&mut app, "Thursday works");
        press(&mut app, KeyCode::Enter);
        let last = app.conversations[1].messages.last().expect("sent");
        assert!(last.from_me);
        assert_eq!(last.body, "Thursday works");

        // The same as Enter on it in Messaging.
        let mut by_hand = App::new();
        by_hand.reply = "a stale draft".to_owned();
        press(&mut by_hand, KeyCode::Char('4'));
        press(&mut by_hand, KeyCode::Char('j'));
        press(&mut by_hand, KeyCode::Enter);
        let mut found = App::new();
        found.reply = "a stale draft".to_owned();
        jump(&mut found, "agenda", 2);
        assert_eq!(snapshot(&found), snapshot(&by_hand));
        assert_eq!(found.reply, by_hand.reply);
        assert_eq!(
            found
                .conversations
                .iter()
                .map(|c| c.unread)
                .collect::<Vec<_>>(),
            by_hand
                .conversations
                .iter()
                .map(|c| c.unread)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn jumping_to_a_comment_or_conversation_from_notifications_ends_the_visit() {
        let new = |app: &App| app.notifications.iter().filter(|n| n.is_new).count();
        for (down, tab) in [(1, Tab::Home), (2, Tab::Messaging)] {
            let mut app = App::new();
            press(&mut app, KeyCode::Char('5'));
            assert_eq!(new(&app), 3);
            jump(&mut app, "typo", down);
            assert_eq!(app.tab, tab);
            assert_eq!(new(&app), 0, "leaving by a jump clears the markers");
            assert_eq!(app.badge(Tab::Notifications), 0);
        }
    }

    #[test]
    fn comments_you_post_or_delete_are_found_or_not() {
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "Zebracorn approved");
        press(&mut app, KeyCode::Enter);
        let hit = SearchHit::Comment {
            post: 0,
            comment: 3,
        };
        assert_eq!(app.search_hits("zebracorn"), [hit]);
        // Your own name doesn't make it a result: authors aren't searched.
        assert!(!app.search_hits("jordan").contains(&hit));

        // Publishing a post moves it down the feed, and the hit follows.
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "New post");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.search_hits("zebracorn"),
            [SearchHit::Comment {
                post: 1,
                comment: 3
            }]
        );
        jump(&mut app, "zebracorn", 0);
        assert_eq!(app.post_sel, 1);
        assert_eq!(app.mode, Mode::PostDetail { comment: 3 });

        // Deleted, it's gone.
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.status, Some("Comment deleted"));
        assert!(app.search_hits("zebracorn").is_empty());
    }

    #[test]
    fn replies_and_shared_posts_make_their_conversation_match() {
        let mut app = App::new();
        assert!(app.search_hits("zebracorn").is_empty());
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "A zebracorn walked in");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.search_hits("zebracorn"),
            [SearchHit::Conversation {
                chat: 2,
                message: Some(2)
            }]
        );

        // Maren Holt's post shared to Felix with `s`.
        let mut app = App::new();
        assert!(app.search_hits("shared a post").is_empty());
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "felix");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Post sent"));
        assert_eq!(
            named_hits(&app, "shared a post"),
            named(&[("conversation", "Felix Abernathy")])
        );
        jump(&mut app, "shared a post", 0);
        assert_eq!((app.tab, app.open_chat), (Tab::Messaging, Some(3)));
    }

    // ------------------------------------------------- Job details scroll

    /// `J` or `K` as the terminal sends it, with Shift held.
    fn shift(app: &mut App, ch: char) {
        app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT));
    }

    /// Jobs, with the selected job's details `max` lines longer than the
    /// card, as the event loop would report for a short terminal.
    fn jobs_overflowing(max: usize) -> App {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        app.fit_job_details(max);
        app
    }

    /// Everything a scroll key must leave alone: every selection, the tab and
    /// mode, the open thread, the jobs' saved and applied flags, every
    /// post's reactions and like state, the people lists, the drafts, the
    /// overlays and the toast.
    type Unscrolled = (
        Snapshot,
        Vec<(bool, bool)>,
        Unsent,
        (usize, usize),
        (String, String),
        (bool, bool, Option<&'static str>, bool),
    );

    fn unscrolled(app: &App) -> Unscrolled {
        (
            snapshot(app),
            app.jobs.iter().map(|j| (j.saved, j.applied)).collect(),
            unsent(app),
            (
                app.invitations.len(),
                app.suggestions.iter().filter(|s| s.pending).count(),
            ),
            (app.draft.clone(), app.comment_draft.clone()),
            (
                app.search.is_some(),
                app.send.is_some(),
                app.status,
                app.should_quit,
            ),
        )
    }

    #[test]
    fn shift_j_and_k_scroll_the_details_one_line_and_stop_at_the_end() {
        let mut app = jobs_overflowing(3);
        let before = unscrolled(&app);
        for want in [1, 2, 3, 3] {
            shift(&mut app, 'J');
            assert_eq!(app.job_scroll, want);
        }
        for want in [2, 1, 0, 0] {
            shift(&mut app, 'K');
            assert_eq!(app.job_scroll, want);
        }
        assert_eq!(unscrolled(&app), before, "only the scroll moves");
    }

    #[test]
    fn k_at_the_top_changes_nothing() {
        let mut app = jobs_overflowing(3);
        let before = unscrolled(&app);
        shift(&mut app, 'K');
        assert_eq!(app.job_scroll, 0);
        assert_eq!(unscrolled(&app), before);
    }

    #[test]
    fn shift_j_and_k_do_nothing_on_other_tabs() {
        for keys in [&['1'][..], &['2'], &['4'], &['5'], &['5', '5']] {
            let mut app = jobs_overflowing(5);
            shift(&mut app, 'J');
            for &key in keys {
                press(&mut app, KeyCode::Char(key));
            }
            let before = unscrolled(&app);
            for ch in "JJKJ".chars() {
                shift(&mut app, ch);
                assert_eq!(app.job_scroll, 1, "{keys:?}");
                assert_eq!(unscrolled(&app), before, "{keys:?}");
            }
            press(&mut app, KeyCode::Char('3'));
            assert_eq!(app.job_scroll, 1, "{keys:?}: kept for Jobs");
        }

        // The post view and an open thread too.
        let mut app = maren_post_view();
        app.fit_job_details(5);
        press(&mut app, KeyCode::Char('j'));
        let before = unscrolled(&app);
        shift(&mut app, 'J');
        shift(&mut app, 'K');
        assert_eq!((app.job_scroll, unscrolled(&app)), (0, before));

        let mut app = App::new();
        app.fit_job_details(5);
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        let before = unscrolled(&app);
        shift(&mut app, 'J');
        assert_eq!((app.job_scroll, unscrolled(&app)), (0, before));
    }

    #[test]
    fn the_first_k_after_overscrolling_moves_the_text() {
        let mut app = jobs_overflowing(4);
        for _ in 0..14 {
            shift(&mut app, 'J');
        }
        assert_eq!(app.job_scroll, 4, "J stops at the end");
        shift(&mut app, 'K');
        assert_eq!(app.job_scroll, 3, "one line up at once");
    }

    #[test]
    fn the_first_k_after_the_screen_grows_moves_the_text() {
        let mut app = jobs_overflowing(6);
        for _ in 0..8 {
            shift(&mut app, 'J');
        }
        assert_eq!(app.job_scroll, 6);
        // The terminal grows: only two lines are hidden now.
        app.fit_job_details(2);
        assert_eq!(app.job_scroll, 2, "pulled back to the new end");
        shift(&mut app, 'K');
        assert_eq!(app.job_scroll, 1);
        shift(&mut app, 'J');
        shift(&mut app, 'J');
        assert_eq!(app.job_scroll, 2);

        // Big enough for everything: back to the top, and J does nothing.
        app.fit_job_details(0);
        assert_eq!(app.job_scroll, 0);
        shift(&mut app, 'J');
        assert_eq!(app.job_scroll, 0);

        // Shrinking keeps the line the card starts on.
        let mut app = jobs_overflowing(3);
        shift(&mut app, 'J');
        app.fit_job_details(9);
        assert_eq!(app.job_scroll, 1);
    }

    #[test]
    fn moving_through_the_jobs_starts_each_at_the_top() {
        for (key, start) in [
            (KeyCode::Char('j'), 0),
            (KeyCode::Down, 0),
            (KeyCode::Char('k'), 2),
            (KeyCode::Up, 2),
        ] {
            let mut app = jobs_overflowing(5);
            while app.job_sel < start {
                press(&mut app, KeyCode::Char('j'));
            }
            shift(&mut app, 'J');
            shift(&mut app, 'J');
            press(&mut app, key);
            assert_ne!(app.job_sel, start, "{key:?}");
            assert_eq!(app.job_scroll, 0, "{key:?}");
        }

        // `j` on the last job and `k` on the first don't change `job_sel`, so
        // they keep the scroll.
        let mut app = jobs_overflowing(5);
        shift(&mut app, 'J');
        press(&mut app, KeyCode::Char('k'));
        assert_eq!((app.job_sel, app.job_scroll), (0, 1));
        while app.job_sel + 1 < app.jobs.len() {
            press(&mut app, KeyCode::Char('j'));
        }
        shift(&mut app, 'J');
        press(&mut app, KeyCode::Char('j'));
        assert_eq!((app.job_sel, app.job_scroll), (app.jobs.len() - 1, 1));
    }

    #[test]
    fn a_search_jump_to_a_job_starts_at_the_top() {
        // From another job on Jobs: `lisbon` is the Senior Rust Engineer.
        let mut app = jobs_overflowing(5);
        press(&mut app, KeyCode::Char('j'));
        shift(&mut app, 'J');
        jump(&mut app, "lisbon", 0);
        assert_eq!((app.tab, app.job_sel, app.job_scroll), (Tab::Jobs, 0, 0));

        // From the same job, and from another tab.
        let mut app = jobs_overflowing(5);
        shift(&mut app, 'J');
        jump(&mut app, "lisbon", 0);
        assert_eq!((app.job_sel, app.job_scroll), (0, 0));

        let mut app = jobs_overflowing(5);
        shift(&mut app, 'J');
        press(&mut app, KeyCode::Char('1'));
        jump(&mut app, "lisbon", 0);
        assert_eq!((app.tab, app.job_sel, app.job_scroll), (Tab::Jobs, 0, 0));

        // A jump anywhere else leaves the Jobs scroll alone.
        let mut app = jobs_overflowing(5);
        shift(&mut app, 'J');
        jump(&mut app, "typo", 0);
        assert_eq!((app.tab, app.job_scroll), (Tab::Home, 1));
    }

    #[test]
    fn the_scroll_survives_tabs_search_saving_and_applying() {
        let mut app = jobs_overflowing(5);
        shift(&mut app, 'J');
        shift(&mut app, 'J');

        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Char('3'));
        assert_eq!((app.job_sel, app.job_scroll), (0, 2), "3 then 1 then 3");
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.job_scroll, 2, "Tab then BackTab");

        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "rust");
        press(&mut app, KeyCode::Esc);
        assert_eq!((app.tab, app.job_scroll), (Tab::Jobs, 2), "/ then Esc");

        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.status, Some("Job unsaved"));
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.status, Some("Job saved"));
        press(&mut app, KeyCode::Char('e'));
        assert_eq!(app.status, Some("Application sent"));
        press(&mut app, KeyCode::Char('e'));
        assert_eq!(app.status, Some("Already applied"));
        assert_eq!((app.job_sel, app.job_scroll), (0, 2), "s and e");
        shift(&mut app, 'K');
        assert_eq!(app.job_scroll, 1, "and K still moves it");
    }

    #[test]
    fn shift_j_and_k_are_typed_in_every_text_box() {
        // Each box opened with the Jobs details scrolled one line.
        let scrolled = || {
            let mut app = jobs_overflowing(5);
            shift(&mut app, 'J');
            app
        };
        let type_jk = |app: &mut App| {
            shift(app, 'J');
            shift(app, 'K');
            assert_eq!(app.job_scroll, 1);
        };

        for tab in ['1', '2', '3', '4', '5'] {
            let mut app = scrolled();
            press(&mut app, KeyCode::Char(tab));
            press(&mut app, KeyCode::Char('/'));
            let before = snapshot(&app);
            type_jk(&mut app);
            assert_eq!(query(&app), Some("JK"), "search on {tab}");
            assert_eq!(snapshot(&app), before, "search on {tab}");
        }

        let mut app = scrolled();
        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Char('p'));
        type_jk(&mut app);
        assert_eq!((app.draft.as_str(), app.mode), ("JK", Mode::Composer));

        let mut app = scrolled();
        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('c'));
        type_jk(&mut app);
        assert_eq!(app.comment_draft, "JK");
        assert_eq!(app.mode, Mode::CommentBox { comment: 0 });

        for keys in [&[][..], &[KeyCode::Enter]] {
            let mut app = scrolled();
            press(&mut app, KeyCode::Char('1'));
            for &key in keys {
                press(&mut app, key);
            }
            press(&mut app, KeyCode::Char('s'));
            type_jk(&mut app);
            assert_eq!(filter(&app), Some("JK"), "picker after {keys:?}");
        }

        let mut app = scrolled();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_jk(&mut app);
        assert_eq!((app.reply.as_str(), app.mode), ("JK", Mode::Reply));
    }
}
