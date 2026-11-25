use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;

use crate::data;
use crate::data::Comment;
use crate::data::Conversation;
use crate::data::Job;
use crate::data::Member;
use crate::data::Message;
use crate::data::News;
use crate::data::Notification;
use crate::data::Person;
use crate::data::Post;
use crate::data::PostId;
use crate::data::Profile;
use crate::data::Suggestion;
use crate::data::Workplace;

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

/// Editable text with a cursor: what all five text boxes (the composer, the
/// comment box, the reply box, search and the Send picker) edit. `cursor` is
/// a byte index into `text`, always on a `char` boundary, so slicing at it
/// never panics. A step is one `char`, so a double-width character such as
/// `日` is one step and never split.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TextField {
    text: String,
    cursor: usize,
}

/// What a key did to a `TextField`, or the key it leaves to the box's owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edit {
    /// The text changed.
    Changed,
    /// The text didn't change. The cursor may have moved.
    Unchanged,
    Enter,
    Esc,
    Up,
    Down,
}

impl TextField {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The cursor, as a byte index into `text()` on a `char` boundary.
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Empty text, cursor at 0: how every box opens.
    fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// The boundary one `char` before the cursor, or 0.
    fn before(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    /// The boundary one `char` after the cursor, or the end.
    fn after(&self) -> usize {
        self.text[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |ch| self.cursor + ch.len_utf8())
    }

    /// Edits the text for one key. A typed key (`KeyCode::Char`, Shift
    /// allowed) goes in at the cursor, which moves past it. `←`/`→` move one
    /// `char` and `Home`/`End` jump to either end. `Backspace` deletes the
    /// `char` before the cursor and `Delete` the one under it, each doing
    /// nothing at its end of the text. Ctrl- and Alt-modified keys, `Tab`,
    /// `BackTab` and every other key do nothing. `Enter`, `Esc`, `↑` and `↓`
    /// are returned for the owner.
    fn on_key(&mut self, key: KeyEvent) -> Edit {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return Edit::Unchanged;
        }
        match key.code {
            KeyCode::Enter => return Edit::Enter,
            KeyCode::Esc => return Edit::Esc,
            KeyCode::Up => return Edit::Up,
            KeyCode::Down => return Edit::Down,
            KeyCode::Left => self.cursor = self.before(),
            KeyCode::Right => self.cursor = self.after(),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.text.len(),
            KeyCode::Backspace if self.cursor > 0 => {
                let start = self.before();
                self.text.replace_range(start..self.cursor, "");
                self.cursor = start;
                return Edit::Changed;
            }
            KeyCode::Delete if self.cursor < self.text.len() => {
                self.text.replace_range(self.cursor..self.after(), "");
                return Edit::Changed;
            }
            KeyCode::Char(ch) => {
                self.text.insert(self.cursor, ch);
                self.cursor += ch.len_utf8();
                return Edit::Changed;
            }
            _ => {}
        }
        Edit::Unchanged
    }
}

/// The text, with the cursor at its end.
impl From<&str> for TextField {
    fn from(text: &str) -> Self {
        TextField {
            text: text.to_owned(),
            cursor: text.len(),
        }
    }
}

/// A one-line text box over a list: what search and the Send picker share.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FilterBox {
    pub query: TextField,
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
    /// Editing the query (`TextField::on_key`) and `↑`/`↓` over a list of
    /// `len` items. A change to the query selects the first item again; a
    /// cursor move, or a `Backspace` or `Delete` that deletes nothing, keeps
    /// the selection. The selection stops at both ends. `Enter` and `Esc` are
    /// returned for the owner to act on.
    fn on_key(&mut self, key: KeyEvent, len: usize) -> Option<BoxKey> {
        match self.query.on_key(key) {
            Edit::Enter => return Some(BoxKey::Enter),
            Edit::Esc => return Some(BoxKey::Esc),
            Edit::Up => self.selected = self.selected.saturating_sub(1),
            Edit::Down => self.selected = (self.selected + 1).min(len.saturating_sub(1)),
            Edit::Changed => self.selected = 0,
            Edit::Unchanged => {}
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

/// One card in the Home feed: post `post`, either as itself or as your repost
/// of it. A repost is a second card for the same post, not a copy, so it always
/// shows the post as it is now. Cards name their post by id, which publishing
/// can't shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Card {
    pub post: PostId,
    pub repost: bool,
}

/// The Jobs tab's filter, one at a time. `f` steps through them in `ALL`'s
/// order, wrapping from `Saved` back to `All`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobFilter {
    All,
    Remote,
    Hybrid,
    OnSite,
    Saved,
}

impl JobFilter {
    pub(crate) const ALL: [JobFilter; 5] = [
        JobFilter::All,
        JobFilter::Remote,
        JobFilter::Hybrid,
        JobFilter::OnSite,
        JobFilter::Saved,
    ];

    /// The pill's text, and the name the list card's title gives it.
    pub(crate) fn label(self) -> &'static str {
        match self {
            JobFilter::All => "All",
            JobFilter::Remote => Workplace::Remote.label(),
            JobFilter::Hybrid => Workplace::Hybrid.label(),
            JobFilter::OnSite => Workplace::OnSite.label(),
            JobFilter::Saved => "Saved",
        }
    }

    /// The filter after this one, as `f` steps.
    fn next(self) -> JobFilter {
        let i = JobFilter::ALL
            .iter()
            .position(|&f| f == self)
            .expect("every filter is listed in JobFilter::ALL");
        JobFilter::ALL[(i + 1) % JobFilter::ALL.len()]
    }

    /// Whether the list shows `job` under this filter. `Saved` reads
    /// `job.saved` as it is now.
    pub(crate) fn matches(self, job: &Job) -> bool {
        match self {
            JobFilter::All => true,
            JobFilter::Remote => job.workplace == Workplace::Remote,
            JobFilter::Hybrid => job.workplace == Workplace::Hybrid,
            JobFilter::OnSite => job.workplace == Workplace::OnSite,
            JobFilter::Saved => job.saved,
        }
    }
}

/// How far someone is from you in your network, as a profile shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Degree {
    First,
    Second,
}

impl Degree {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Degree::First => "1st",
            Degree::Second => "2nd",
        }
    }
}

/// The action a profile's top card offers, from the current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileAction {
    /// Their pending invitation, `invitations[i]`: Accept / Ignore.
    Invitation(usize),
    /// `suggestions[i]`: + Connect, or Pending once you've sent one.
    Suggestion(usize),
    /// Your conversation with them, `conversations[i]`: Message.
    Message(usize),
}

/// A profile page, open over whatever tab and mode it was opened from. Like
/// search, it leaves those untouched underneath, so Esc puts everything back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProfilePage {
    /// Whose profile: a member's name, or your own.
    pub name: &'static str,
    /// The selected Activity post, an index into `App::activity(name)`.
    pub post: usize,
}

/// The selected row on My Network, which spans both sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkItem {
    Invitation(usize),
    Suggestion(usize),
}

pub(crate) struct App {
    pub profile: Profile,
    /// Every post once, newest first. Search goes through these.
    pub posts: Vec<Post>,
    /// The Home feed, top to bottom: one card per post, in the order of
    /// `posts`, with your reposts in between.
    pub feed: Vec<Card>,
    pub news: Vec<News>,
    pub invitations: Vec<Person>,
    pub suggestions: Vec<Suggestion>,
    pub jobs: Vec<Job>,
    pub conversations: Vec<Conversation>,
    pub notifications: Vec<Notification>,
    /// One profile per person other than you, by name.
    pub members: Vec<Member>,
    pub tab: Tab,
    pub mode: Mode,
    /// Index into `feed`.
    pub post_sel: usize,
    /// Index into invitations followed by suggestions, as one list.
    pub network_sel: usize,
    /// Index into `jobs`, not into the filtered list, so the selection
    /// follows the job when the filter changes. It is the selected job only
    /// while `job_filter` shows it (`selected_job`); every key that changes
    /// the filter or what it shows moves it onto a shown job, or leaves it
    /// when the list is empty.
    pub job_sel: usize,
    /// Which jobs the Jobs list shows. Kept like a selection.
    pub job_filter: JobFilter,
    /// Lines of the Jobs details card scrolled off the top, at most
    /// `job_scroll_max`. Back to 0 whenever the selected job changes.
    pub job_scroll: usize,
    /// How far the details card can scroll, set by `fit_job_details`.
    pub job_scroll_max: usize,
    pub chat_sel: usize,
    pub notif_sel: usize,
    /// The conversation shown on the right of Messaging, opened with Enter.
    pub open_chat: Option<usize>,
    /// Composer text and cursor.
    pub draft: TextField,
    /// Reply box text and cursor.
    pub reply: TextField,
    /// The post view's comment box text and cursor.
    pub comment_draft: TextField,
    /// `Some` while the search overlay is open. Takes every key but Ctrl-C.
    pub search: Option<Search>,
    /// `Some` while the Send picker is open. Takes every key but Ctrl-C.
    pub send: Option<SendPicker>,
    /// `Some` while a profile page is open over the current tab and mode.
    pub profile_page: Option<ProfilePage>,
    pub status: Option<&'static str>,
    pub should_quit: bool,
}

impl App {
    pub(crate) fn new() -> Self {
        let posts = data::posts();
        let feed = posts
            .iter()
            .map(|p| Card {
                post: p.id,
                repost: false,
            })
            .collect();
        App {
            profile: data::profile(),
            posts,
            feed,
            news: data::news(),
            invitations: data::invitations(),
            suggestions: data::suggestions(),
            jobs: data::jobs(),
            conversations: data::conversations(),
            notifications: data::notifications(),
            members: data::members(),
            tab: Tab::Home,
            mode: Mode::Browse,
            post_sel: 0,
            network_sel: 0,
            job_sel: 0,
            job_filter: JobFilter::All,
            job_scroll: 0,
            job_scroll_max: 0,
            chat_sel: 0,
            notif_sel: 0,
            open_chat: None,
            draft: TextField::default(),
            reply: TextField::default(),
            comment_draft: TextField::default(),
            search: None,
            send: None,
            profile_page: None,
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

    /// The jobs the Jobs list shows under `job_filter`, as indexes into
    /// `jobs`, in seed order.
    pub(crate) fn shown_jobs(&self) -> Vec<usize> {
        (0..self.jobs.len())
            .filter(|&i| self.job_filter.matches(&self.jobs[i]))
            .collect()
    }

    /// The selected job, as an index into `jobs`: `job_sel` while the list
    /// shows it. `None` only when the list is empty, where `s`, `e`, `j`,
    /// `k`, `J` and `K` do nothing.
    pub(crate) fn selected_job(&self) -> Option<usize> {
        self.jobs
            .get(self.job_sel)
            .is_some_and(|job| self.job_filter.matches(job))
            .then_some(self.job_sel)
    }

    /// Switches the Jobs list to `filter`. The selected job stays selected if
    /// `filter` shows it, otherwise the first job it shows is selected. The
    /// details go back to the top only when that changes the selected job.
    fn set_job_filter(&mut self, filter: JobFilter) {
        let before = self.selected_job();
        self.job_filter = filter;
        let shown = self.shown_jobs();
        match before {
            Some(i) if shown.contains(&i) => {}
            _ => self.job_sel = shown.first().copied().unwrap_or(self.job_sel),
        }
        self.reset_job_scroll_if_moved(before);
    }

    /// The details card starts at the top whenever the selected job changes.
    fn reset_job_scroll_if_moved(&mut self, before: Option<usize>) {
        if self.selected_job() != before {
            self.job_scroll = 0;
        }
    }

    /// The post with id `id`, wherever publishing has moved it in `posts`.
    pub(crate) fn post(&self, id: PostId) -> Option<&Post> {
        self.posts.iter().find(|p| p.id == id)
    }

    /// The post behind the selected card: for a repost, the original.
    pub(crate) fn selected_post(&self) -> Option<&Post> {
        self.feed.get(self.post_sel).and_then(|c| self.post(c.post))
    }

    fn selected_post_mut(&mut self) -> Option<&mut Post> {
        let id = self.feed.get(self.post_sel)?.post;
        self.posts.iter_mut().find(|p| p.id == id)
    }

    /// Whether comment `index` of the selected post is yours, so `x` in the
    /// post view deletes it (and its footer says so).
    pub(crate) fn is_yours(&self, index: usize) -> bool {
        self.selected_post()
            .and_then(|p| p.comments.get(index))
            .is_some_and(|c| c.author == self.profile.name)
    }

    /// Whether you've reposted post `id`. Computed from the feed, never stored.
    pub(crate) fn reposted(&self, id: PostId) -> bool {
        self.feed.contains(&Card {
            post: id,
            repost: true,
        })
    }

    /// The feed index of post `posts[i]`'s own card, not a repost of it.
    fn card_of(&self, i: usize) -> Option<usize> {
        let id = self.posts.get(i)?.id;
        self.feed.iter().position(|&c| {
            c == Card {
                post: id,
                repost: false,
            }
        })
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

    /// The profile of `name`, someone other than you.
    pub(crate) fn member(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.name == name)
    }

    /// `name`'s posts, newest first, as indexes into `posts`.
    pub(crate) fn activity(&self, name: &str) -> Vec<usize> {
        (0..self.posts.len())
            .filter(|&i| self.posts[i].author == name)
            .collect()
    }

    /// `name`'s degree, from whether they are your connection now. `None` for
    /// you.
    pub(crate) fn degree(&self, name: &str) -> Option<Degree> {
        let member = self.member(name)?;
        Some(if member.connected {
            Degree::First
        } else {
            Degree::Second
        })
    }

    /// `name`'s pending invitation or suggestion on My Network, if any.
    fn network_item_of(&self, name: &str) -> Option<NetworkItem> {
        if let Some(i) = self.invitations.iter().position(|p| p.name == name) {
            return Some(NetworkItem::Invitation(i));
        }
        let i = self
            .suggestions
            .iter()
            .position(|s| s.person.name == name)?;
        Some(NetworkItem::Suggestion(i))
    }

    /// Your conversation with `name`, as an index into `conversations`.
    fn conversation_of(&self, name: &str) -> Option<usize> {
        self.conversations.iter().position(|c| c.name == name)
    }

    /// What `name`'s top card offers: their invitation, the suggestion to
    /// connect, or your conversation. Nothing for you.
    pub(crate) fn profile_action(&self, name: &str) -> Option<ProfileAction> {
        if name == self.profile.name {
            return None;
        }
        match self.network_item_of(name) {
            Some(NetworkItem::Invitation(i)) => Some(ProfileAction::Invitation(i)),
            Some(NetworkItem::Suggestion(i)) => Some(ProfileAction::Suggestion(i)),
            None => self.conversation_of(name).map(ProfileAction::Message),
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

    /// Switching tabs closes the post view and the profile page. Opening Notifications marks them all
    /// read, which clears the badge, but the ones that were unread stay marked
    /// as new until you leave the tab.
    fn set_tab(&mut self, tab: Tab) {
        let entering = tab == Tab::Notifications && self.tab != Tab::Notifications;
        let leaving = self.tab == Tab::Notifications && tab != Tab::Notifications;
        self.tab = tab;
        self.mode = Mode::Browse;
        self.profile_page = None;
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
            (Tab::Home, _) => step(&mut self.post_sel, self.feed.len(), delta),
            (Tab::Network, _) => {
                let len = self.network_len();
                step(&mut self.network_sel, len, delta);
            }
            (Tab::Jobs, _) => {
                // Through the shown jobs only. With none, nothing moves.
                let shown = self.shown_jobs();
                let before = self.selected_job();
                let Some(mut row) = shown.iter().position(|&i| Some(i) == before) else {
                    return;
                };
                step(&mut row, shown.len(), delta);
                self.job_sel = shown[row];
                self.reset_job_scroll_if_moved(before);
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
        if self.profile_page.is_some() {
            self.on_profile_key(key.code);
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
            Some(search) => self.search_hits(search.query.text()),
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
                // Search ignores the filter, so the job may be hidden: then
                // the list goes back to all jobs, so the jump shows it.
                if !self.jobs.get(i).is_some_and(|j| self.job_filter.matches(j)) {
                    self.job_filter = JobFilter::All;
                }
                self.job_sel = i;
                self.job_scroll = 0;
            }
            SearchHit::Post(i) => self.go_to_post(i, Mode::Browse),
            SearchHit::Comment { post, comment } => {
                self.go_to_post(post, Mode::PostDetail { comment });
            }
            SearchHit::Conversation { chat, .. } => self.go_to_conversation(chat),
        }
    }

    /// Home, on post `posts[i]`'s own card (never a repost of it), in `mode`.
    fn go_to_post(&mut self, i: usize, mode: Mode) {
        self.set_tab(Tab::Home);
        self.post_sel = self.card_of(i).unwrap_or(self.post_sel);
        self.mode = mode;
    }

    /// Messaging, with conversation `chat` opened as Enter on it opens it.
    fn go_to_conversation(&mut self, chat: usize) {
        self.set_tab(Tab::Messaging);
        self.chat_sel = chat;
        self.open_conversation();
    }

    /// Opens `name`'s profile page over the current tab and mode, with its
    /// first Activity post selected.
    fn open_profile(&mut self, name: &'static str) {
        if name == self.profile.name || self.member(name).is_some() {
            self.profile_page = Some(ProfilePage { name, post: 0 });
        }
    }

    /// Keys on a profile page. Like the browse keys, except that `j`/`k` move
    /// through Activity, and `a`, `x`, `c` and `m` act on the person.
    fn on_profile_key(&mut self, code: KeyCode) {
        let Some(page) = self.profile_page else {
            return;
        };
        let posts = self.activity(page.name);
        let mut step = |delta: isize| {
            let post = page
                .post
                .saturating_add_signed(delta)
                .min(posts.len().saturating_sub(1));
            self.profile_page = Some(ProfilePage { post, ..page });
        };
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char(ch @ '1'..='5') => self.set_tab(Tab::ALL[usize::from(ch as u8 - b'1')]),
            KeyCode::Tab => self.set_tab(self.tab.cycle(true)),
            KeyCode::BackTab => self.set_tab(self.tab.cycle(false)),
            KeyCode::Char('j') | KeyCode::Down => step(1),
            KeyCode::Char('k') | KeyCode::Up => step(-1),
            KeyCode::Esc => self.profile_page = None,
            KeyCode::Char('/') => self.search = Some(Search::default()),
            KeyCode::Enter => {
                if let Some(&i) = posts.get(page.post) {
                    self.go_to_post(i, Mode::PostDetail { comment: 0 });
                }
            }
            KeyCode::Char('a' | 'x' | 'c') => {
                self.act_on_network_item(code, self.network_item_of(page.name));
            }
            KeyCode::Char('m') => {
                if let Some(chat) = self.conversation_of(page.name) {
                    self.go_to_conversation(chat);
                }
            }
            _ => {}
        }
    }

    fn on_home_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('s') if self.selected_post().is_some() => {
                self.send = Some(SendPicker::default());
            }
            KeyCode::Char('l') => {
                if let Some(post) = self.selected_post_mut() {
                    post.liked = !post.liked;
                    if post.liked {
                        post.reactions += 1;
                    } else {
                        post.reactions -= 1;
                    }
                }
            }
            KeyCode::Enter if self.mode == Mode::Browse && self.selected_post().is_some() => {
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
            KeyCode::Char('r') => self.repost(),
            KeyCode::Char('v') => {
                if let Some(author) = self.selected_post().map(|p| p.author) {
                    self.open_profile(author);
                }
            }
            _ => {}
        }
    }

    /// Puts a repost card for the selected post at the top of the feed. The
    /// selection moves down with the card it was on, so the feed and the post
    /// view stay on the same post. A repost card stands for its original, so
    /// reposting it reposts the original, and each post is reposted at most
    /// once.
    fn repost(&mut self) {
        let Some(&Card { post, .. }) = self.feed.get(self.post_sel) else {
            return;
        };
        if self.reposted(post) {
            self.status = Some("Already reposted");
            return;
        }
        self.feed.insert(0, Card { post, repost: true });
        self.post_sel += 1;
        self.status = Some("Reposted");
    }

    /// The Send picker is a text box, like search: Ctrl- and Alt-modified keys
    /// do nothing at all.
    fn on_send_key(&mut self, key: KeyEvent) {
        let recipients = match &self.send {
            Some(picker) => self.send_recipients(picker.query.text()),
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
        let Some(post) = self.selected_post() else {
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
        if !self.is_yours(index) {
            return;
        }
        let Some(post) = self.selected_post_mut() else {
            return;
        };
        post.comments.remove(index);
        self.mode = Mode::PostDetail {
            comment: index.min(post.comments.len().saturating_sub(1)),
        };
        self.status = Some("Comment deleted");
    }

    /// The comment box is a text box, like search (`TextField::on_key`).
    /// `comment` is the comment selected before `c`.
    fn on_comment_key(&mut self, key: KeyEvent, comment: usize) {
        match self.comment_draft.on_key(key) {
            Edit::Esc => {
                self.comment_draft.clear();
                self.mode = Mode::PostDetail { comment };
            }
            Edit::Enter => self.post_comment(),
            Edit::Changed | Edit::Unchanged | Edit::Up | Edit::Down => {}
        }
    }

    /// Appends the comment box text to the open post as the newest comment,
    /// and selects it.
    fn post_comment(&mut self) {
        let body = self.comment_draft.text().trim();
        if body.is_empty() {
            self.status = Some("Write something first");
            return;
        }
        let (author, headline) = (self.profile.name, self.profile.headline);
        let body = body.to_owned();
        let Some(post) = self.selected_post_mut() else {
            return;
        };
        post.comments.push(Comment {
            author,
            headline,
            hours_ago: 0,
            body,
        });
        self.mode = Mode::PostDetail {
            comment: post.comments.len() - 1,
        };
        self.comment_draft.clear();
        self.status = Some("Comment posted");
    }

    /// The composer is a text box, like search (`TextField::on_key`).
    fn on_composer_key(&mut self, key: KeyEvent) {
        match self.draft.on_key(key) {
            Edit::Esc => {
                self.draft.clear();
                self.mode = Mode::Browse;
            }
            Edit::Enter => self.publish(),
            Edit::Changed | Edit::Unchanged | Edit::Up | Edit::Down => {}
        }
    }

    fn publish(&mut self) {
        let body = self.draft.text().trim();
        if body.is_empty() {
            self.status = Some("Write something first");
            return;
        }
        // One past the highest id, so it can't be any other post's.
        let id = PostId(self.posts.iter().map(|p| p.id.0 + 1).max().unwrap_or(0));
        self.posts.insert(
            0,
            Post {
                id,
                author: self.profile.name,
                headline: self.profile.headline,
                hours_ago: 0,
                body: body.to_owned(),
                reactions: 0,
                comments: Vec::new(),
                liked: false,
            },
        );
        self.feed.insert(
            0,
            Card {
                post: id,
                repost: false,
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
            (KeyCode::Char('v') | KeyCode::Enter, Some(NetworkItem::Invitation(i))) => {
                self.open_profile(self.invitations[i].name);
            }
            (KeyCode::Char('v') | KeyCode::Enter, Some(NetworkItem::Suggestion(i))) => {
                self.open_profile(self.suggestions[i].person.name);
            }
            _ => self.act_on_network_item(code, item),
        }
    }

    /// `a`, `x` or `c` on `item`, from My Network or the person's profile:
    /// accept or ignore an invitation, or connect to or withdraw from a
    /// suggestion. Anything else on either does nothing. Afterwards the My
    /// Network selection is clamped to the list.
    fn act_on_network_item(&mut self, code: KeyCode, item: Option<NetworkItem>) {
        match (code, item) {
            (KeyCode::Char('a'), Some(NetworkItem::Invitation(i))) => {
                let person = self.invitations.remove(i);
                self.profile.connections += 1;
                if let Some(m) = self.members.iter_mut().find(|m| m.name == person.name) {
                    m.connected = true;
                }
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
        if code == KeyCode::Char('f') {
            self.set_job_filter(self.job_filter.next());
            return;
        }
        // An empty list has no selected job, so the rest do nothing.
        let Some(i) = self.selected_job() else {
            return;
        };
        let job = &mut self.jobs[i];
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
                self.drop_unsaved_job(i);
            }
            KeyCode::Char('e') if job.applied => self.status = Some("Already applied"),
            KeyCode::Char('e') => {
                job.applied = true;
                self.status = Some("Application sent");
            }
            _ => {}
        }
    }

    /// After `s` on `jobs[i]`: under `Saved`, an unsaved job leaves the list
    /// at once, and the selection keeps its row, clamped to the list, as
    /// removing an invitation keeps `network_sel`. The next job slides under
    /// the cursor, or the cursor moves up when the last one goes. Under any
    /// other filter the list doesn't change, so neither does the selection.
    fn drop_unsaved_job(&mut self, i: usize) {
        if self.job_filter != JobFilter::Saved || self.jobs[i].saved {
            return;
        }
        // The row it had, counting the jobs before it that are still shown.
        let row = (0..i)
            .filter(|&j| self.job_filter.matches(&self.jobs[j]))
            .count();
        let shown = self.shown_jobs();
        if let Some(&next) = shown.get(row.min(shown.len().saturating_sub(1))) {
            self.job_sel = next;
        }
        self.reset_job_scroll_if_moved(Some(i));
    }

    fn on_messaging_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => self.open_conversation(),
            KeyCode::Char('v') => {
                if let Some(name) = self.conversations.get(self.chat_sel).map(|c| c.name) {
                    self.open_profile(name);
                }
            }
            _ => {}
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

    /// The reply box is a text box, like search (`TextField::on_key`).
    fn on_reply_key(&mut self, key: KeyEvent) {
        match self.reply.on_key(key) {
            Edit::Esc => {
                self.reply.clear();
                self.mode = Mode::Browse;
            }
            Edit::Enter => {
                let body = self.reply.text().trim();
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
            Edit::Changed | Edit::Unchanged | Edit::Up | Edit::Down => {}
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
        app.search.as_ref().map(|s| s.query.text())
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
        assert_eq!(app.comment_draft.text(), "q1jklcp/ 5");
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
        assert_eq!(app.comment_draft.text(), "q1jklcp/ 5Q");

        // `←`/`→` move the cursor and change nothing else.
        press(&mut app, KeyCode::Left);
        assert_eq!(app.comment_draft.cursor(), 10, "before the Q");
        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.comment_draft.cursor(), 10);
        assert_eq!(app.comment_draft.text(), "q1jklcp/ 5Q");
        assert_eq!(snapshot(&app), before, "no comment moved");
        press(&mut app, KeyCode::Right);
        assert_eq!(app.comment_draft.cursor(), 11, "back at the end");

        // Tab, BackTab, `↑`/`↓` and Ctrl- or Alt-modified keys do nothing.
        for code in [KeyCode::Tab, KeyCode::BackTab, KeyCode::Up, KeyCode::Down] {
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
            (KeyCode::Left, KeyModifiers::CONTROL),
            (KeyCode::Home, KeyModifiers::ALT),
        ] {
            app.on_key(KeyEvent::new(code, modifiers));
        }
        assert_eq!(app.comment_draft.text(), "q1jklcp/ 5Q");
        assert_eq!(app.comment_draft.cursor(), 11);
        assert_eq!(snapshot(&app), before);
        assert_eq!(app.posts[0].comments.len(), comments);
        assert!(app.status.is_none());

        // Backspace deletes, and on an empty draft keeps focus.
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.comment_draft.text(), "q1jklcp/ 5");
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
        assert_eq!(app.comment_draft.text(), "x-ray");
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
        assert_eq!(app.comment_draft.text(), "   ", "and the draft");
        assert_eq!(app.posts[0].comments.len(), 3);

        // The toast clears on the next key, which is typed.
        type_str(&mut app, "ok");
        assert!(app.status.is_none());
        assert_eq!(app.comment_draft.text(), "   ok");
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
        assert_eq!(app.draft.text(), "q1jl axcse");
        assert!(!app.should_quit, "q is typed, not quit");
        assert_eq!(app.tab, Tab::Home, "1 is typed, not a tab switch");
        assert_eq!(app.post_sel, 0, "j is typed, not a move");
        assert_eq!(app.posts[0].reactions, reactions, "l is typed, not a like");
        assert!(!app.posts[0].liked);

        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.draft.text(), "q1jl axcs");

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
        assert_eq!(app.draft.text(), "a/b");
        assert!(app.search.is_none());
        assert_eq!(app.mode, Mode::Composer);

        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "and/or");
        assert_eq!(app.reply.text(), "and/or");
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
        app.send.as_ref().map(|p| p.query.text())
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
        TextField,
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
        assert_eq!((app.draft.text(), app.mode), ("s", Mode::Composer));
        assert!(app.send.is_none());

        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "s");
        assert_eq!(app.comment_draft.text(), "s");
        assert_eq!(app.mode, Mode::CommentBox { comment: 0 });
        assert!(app.send.is_none());

        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "s");
        assert_eq!((app.reply.text(), app.mode), ("s", Mode::Reply));
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
        app.reply = TextField::from("half a reply");
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
        assert_eq!(app.reply.text(), "half a reply");
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
        app.reply = TextField::from("a stale draft");
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
        by_hand.reply = TextField::from("a stale draft");
        press(&mut by_hand, KeyCode::Char('4'));
        press(&mut by_hand, KeyCode::Char('j'));
        press(&mut by_hand, KeyCode::Enter);
        let mut found = App::new();
        found.reply = TextField::from("a stale draft");
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
        (TextField, TextField),
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
        assert_eq!((app.draft.text(), app.mode), ("JK", Mode::Composer));

        let mut app = scrolled();
        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('c'));
        type_jk(&mut app);
        assert_eq!(app.comment_draft.text(), "JK");
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
        assert_eq!((app.reply.text(), app.mode), ("JK", Mode::Reply));
    }

    /// The author of the post behind each feed card, `+` marking a repost.
    fn feed_authors(app: &App) -> Vec<String> {
        app.feed
            .iter()
            .map(|card| {
                let author = app.post(card.post).expect("every card has its post").author;
                format!("{}{author}", if card.repost { "+" } else { "" })
            })
            .collect()
    }

    /// Everything that `r` must leave alone when it does nothing.
    fn unreposted(app: &App) -> (Snapshot, Vec<Card>, Vec<(u32, bool, usize)>) {
        (
            snapshot(app),
            app.feed.clone(),
            app.posts
                .iter()
                .map(|p| (p.reactions, p.liked, p.comments.len()))
                .collect(),
        )
    }

    fn select_card(app: &mut App, author: &str, repost: bool) {
        let i = app
            .feed
            .iter()
            .position(|c| c.repost == repost && app.post(c.post).map(|p| p.author) == Some(author))
            .unwrap_or_else(|| panic!("{author} is in the feed: {:?}", feed_authors(app)));
        while app.post_sel > i {
            press(app, KeyCode::Char('k'));
        }
        while app.post_sel < i {
            press(app, KeyCode::Char('j'));
        }
    }

    /// What keeps `feed` and `posts` in step: every post has exactly one card
    /// of its own, those cards come in `posts` order, and every repost card
    /// names a post in `posts`, at most one repost card per post.
    fn assert_feed_matches_posts(app: &App) {
        let own: Vec<PostId> = app
            .feed
            .iter()
            .filter(|c| !c.repost)
            .map(|c| c.post)
            .collect();
        let ids: Vec<PostId> = app.posts.iter().map(|p| p.id).collect();
        assert_eq!(own, ids, "one own card per post, in order");
        let mut reposts: Vec<PostId> = Vec::new();
        for card in app.feed.iter().filter(|c| c.repost) {
            assert!(ids.contains(&card.post), "{card:?} names a post");
            assert!(!reposts.contains(&card.post), "{card:?} is the only repost");
            reposts.push(card.post);
        }
    }

    #[test]
    fn post_ids_are_unique() {
        let unique = |app: &App| {
            let ids: Vec<PostId> = app.posts.iter().map(|p| p.id).collect();
            for (i, id) in ids.iter().enumerate() {
                assert!(!ids[..i].contains(id), "{id:?} twice in {ids:?}");
            }
            ids.len()
        };
        let mut app = App::new();
        assert_eq!(unique(&app), 9);
        for body in ["hello", "hello again"] {
            press(&mut app, KeyCode::Char('p'));
            type_str(&mut app, body);
            press(&mut app, KeyCode::Enter);
            assert_eq!(app.status, Some("Post published"));
        }
        assert_eq!(unique(&app), 11);
        assert_eq!(app.posts[0].body, "hello again");
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn r_reposts_to_the_top_and_keeps_the_selection_in_the_feed() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        let posts = app.posts.len();
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.status, Some("Reposted"));
        assert_eq!(
            app.feed[0],
            Card {
                post: app.posts[1].id,
                repost: true
            }
        );
        assert_eq!(
            &feed_authors(&app)[..3],
            [
                "+Tobias Ferreira-Lund",
                "Maren Holt",
                "Tobias Ferreira-Lund"
            ]
        );
        // The same post, one card lower, and still its own card.
        assert_eq!((app.mode, app.post_sel), (Mode::Browse, 2));
        assert!(!app.feed[app.post_sel].repost);
        assert_eq!(
            app.selected_post().map(|p| p.author),
            Some("Tobias Ferreira-Lund")
        );
        assert_eq!(app.posts.len(), posts, "a repost is not a new post");
        assert!(app.reposted(app.posts[1].id));
        assert!(!app.reposted(app.posts[0].id));
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn r_in_the_post_view_keeps_the_post_and_comment() {
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.status, Some("Reposted"));
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        assert_eq!(app.post_sel, 1);
        assert_eq!(app.selected_post().map(|p| p.author), Some("Maren Holt"));
        assert_eq!(&feed_authors(&app)[..2], ["+Maren Holt", "Maren Holt"]);
        // `k` carries on from the same comment, and Esc lands on the original.
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.mode, Mode::PostDetail { comment: 1 });
        press(&mut app, KeyCode::Esc);
        assert_eq!((app.mode, app.post_sel), (Mode::Browse, 1));
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn r_does_nothing_on_other_tabs() {
        for tab in ['2', '3', '4', '5'] {
            let mut app = App::new();
            press(&mut app, KeyCode::Char(tab));
            let before = unreposted(&app);
            let jobs: Vec<(bool, bool)> = app.jobs.iter().map(|j| (j.saved, j.applied)).collect();
            let people = (app.invitations.len(), app.profile.connections);
            press(&mut app, KeyCode::Char('r'));
            assert_eq!(unreposted(&app), before, "r on {tab}");
            assert_eq!(app.status, None, "r on {tab}");
            assert_eq!(
                app.jobs
                    .iter()
                    .map(|j| (j.saved, j.applied))
                    .collect::<Vec<_>>(),
                jobs
            );
            assert_eq!((app.invitations.len(), app.profile.connections), people);
            assert_eq!(app.job_scroll, 0);
            assert_feed_matches_posts(&app);
        }
    }

    #[test]
    fn a_post_is_reposted_once() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('r'));
        let before = unreposted(&app);
        // Again on the original.
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.status, Some("Already reposted"));
        assert_eq!(unreposted(&app), before);
        // On the repost, which stands for the original.
        press(&mut app, KeyCode::Char('k'));
        let before = unreposted(&app);
        assert_eq!(app.post_sel, 0);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.status, Some("Already reposted"));
        assert_eq!(unreposted(&app), before);
        // And from the repost's post view.
        press(&mut app, KeyCode::Enter);
        let before = unreposted(&app);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.status, Some("Already reposted"));
        assert_eq!(unreposted(&app), before);
        assert_eq!(app.feed.iter().filter(|c| c.repost).count(), 1);
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn your_own_post_can_be_reposted() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "hello");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.status, Some("Reposted"));
        assert_eq!(
            &feed_authors(&app)[..3],
            ["+Jordan Vale", "Jordan Vale", "Maren Holt"]
        );
        assert_eq!(app.post_sel, 1);
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn likes_and_comments_from_a_repost_change_the_original() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('r'));
        press(&mut app, KeyCode::Char('k'));
        assert!(app.feed[app.post_sel].repost);

        press(&mut app, KeyCode::Char('l'));
        assert_eq!((app.posts[0].reactions, app.posts[0].liked), (343, true));
        // Unliking from the original undoes it.
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('l'));
        assert_eq!((app.posts[0].reactions, app.posts[0].liked), (342, false));
        press(&mut app, KeyCode::Char('l'));
        press(&mut app, KeyCode::Char('k'));

        // Comment from the repost's post view.
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "From the repost");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Comment posted"));
        assert_eq!(app.posts[0].comments.len(), 4);
        assert_eq!(app.posts[0].comments[3].body, "From the repost");
        assert_eq!(app.mode, Mode::PostDetail { comment: 3 });
        assert_eq!(app.selected_post().map(|p| p.comments.len()), Some(4));
        // The original's post view has it too.
        press(&mut app, KeyCode::Esc);
        assert_eq!(
            (app.mode, app.post_sel),
            (Mode::Browse, 0),
            "back on the repost"
        );
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.selected_post().map(|p| p.comments.len()), Some(4));
        press(&mut app, KeyCode::Esc);

        // Delete it from the repost's post view.
        press(&mut app, KeyCode::Char('k'));
        press(&mut app, KeyCode::Enter);
        for _ in 0..3 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.status, Some("Comment deleted"));
        assert_eq!(app.posts[0].comments.len(), 3);
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });
        // Nothing else moved: still one repost, still liked.
        assert_eq!((app.posts[0].reactions, app.posts[0].liked), (343, true));
        assert_eq!(&feed_authors(&app)[..2], ["+Maren Holt", "Maren Holt"]);
        assert_eq!(
            app.posts
                .iter()
                .skip(1)
                .map(|p| p.comments.len())
                .collect::<Vec<_>>(),
            App::new()
                .posts
                .iter()
                .skip(1)
                .map(|p| p.comments.len())
                .collect::<Vec<_>>()
        );
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn reposts_keep_their_original_when_cards_go_in_above() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('r'));
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "hello");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.post_sel, 0);
        assert_feed_matches_posts(&app);
        select_card(&mut app, "Lucia Brandt", false);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.status, Some("Reposted"));
        assert_eq!(app.selected_post().map(|p| p.author), Some("Lucia Brandt"));
        assert!(!app.feed[app.post_sel].repost);
        assert_eq!(
            feed_authors(&app),
            [
                "+Lucia Brandt",
                "Jordan Vale",
                "+Maren Holt",
                "Maren Holt",
                "Tobias Ferreira-Lund",
                "Ines Okonkwo",
                "Casey Lindqvist",
                "Ravi Tenorio",
                "Hana Sorensen",
                "Felix Abernathy",
                "Lucia Brandt",
                "Wendell Achterberg",
            ]
        );
        // Each repost is the very post, not one at the index it once had.
        let id = |author: &str| app.posts.iter().find(|p| p.author == author).map(|p| p.id);
        assert_eq!(Some(app.feed[0].post), id("Lucia Brandt"));
        assert_eq!(Some(app.feed[2].post), id("Maren Holt"));
        assert!(app.reposted(app.feed[3].post) && !app.reposted(app.feed[1].post));
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn search_finds_a_reposted_post_once_and_lands_on_the_original() {
        let mut app = App::new();
        select_card(&mut app, "Felix Abernathy", false);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.post_sel, 7);
        // The original is post 6 still, its card 7.
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
        assert_eq!(
            named_hits(&app, "typo"),
            named(&[
                ("post", "Felix Abernathy"),
                ("comment", "Maren Holt on Felix Abernathy"),
                ("conversation", "Felix Abernathy"),
            ])
        );

        // From the top of the feed, the post jump lands on the original card.
        select_card(&mut app, "Maren Holt", false);
        jump(&mut app, "typo", 0);
        assert_eq!(
            (app.tab, app.mode, app.post_sel),
            (Tab::Home, Mode::Browse, 7)
        );
        assert!(!app.feed[7].repost);
        assert_eq!(
            app.selected_post().map(|p| p.author),
            Some("Felix Abernathy")
        );

        // So does the comment jump, from the repost's post view.
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('1'));
        select_card(&mut app, "Felix Abernathy", true);
        press(&mut app, KeyCode::Enter);
        jump(&mut app, "typo", 1);
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });
        assert_eq!(app.post_sel, 7);
        press(&mut app, KeyCode::Esc);
        assert_eq!((app.mode, app.post_sel), (Mode::Browse, 7));
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn s_on_a_repost_sends_the_original() {
        let send_to_felix = |app: &mut App| {
            press(app, KeyCode::Char('s'));
            type_str(app, "felix");
            press(app, KeyCode::Enter);
            assert_eq!(app.status, Some("Post sent"));
            app.conversations[3]
                .messages
                .last()
                .expect("sent")
                .body
                .clone()
        };
        let mut app = App::new();
        press(&mut app, KeyCode::Char('r'));
        press(&mut app, KeyCode::Char('k'));
        assert!(app.feed[app.post_sel].repost);
        assert_eq!(send_to_felix(&mut app), MAREN_SHARED);
        assert_eq!(app.post_sel, 0, "back on the repost");
        // From the repost's post view too, and the same as from the original.
        press(&mut app, KeyCode::Enter);
        assert_eq!(send_to_felix(&mut app), MAREN_SHARED);
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(send_to_felix(&mut app), MAREN_SHARED);
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn r_is_typed_in_every_text_box() {
        let reposted = |app: &App| app.feed.iter().any(|c| c.repost);

        for tab in ['1', '2', '3', '4', '5'] {
            let mut app = App::new();
            press(&mut app, KeyCode::Char(tab));
            press(&mut app, KeyCode::Char('/'));
            let before = unreposted(&app);
            press(&mut app, KeyCode::Char('r'));
            assert_eq!(query(&app), Some("r"), "search on {tab}");
            assert_eq!(unreposted(&app), before, "search on {tab}");
            assert_eq!(app.status, None);
            assert_feed_matches_posts(&app);
        }
        // Over the post view too.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(query(&app), Some("r"));
        assert!(!reposted(&app));
        assert_feed_matches_posts(&app);

        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        press(&mut app, KeyCode::Char('r'));
        assert_eq!((app.draft.text(), app.mode), ("r", Mode::Composer));
        assert!(!reposted(&app));
        assert_feed_matches_posts(&app);

        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.comment_draft.text(), "r");
        assert_eq!(app.mode, Mode::CommentBox { comment: 0 });
        assert!(!reposted(&app));
        assert_feed_matches_posts(&app);

        for keys in [&[][..], &[KeyCode::Enter]] {
            let mut app = App::new();
            for &key in keys {
                press(&mut app, key);
            }
            press(&mut app, KeyCode::Char('s'));
            press(&mut app, KeyCode::Char('r'));
            assert_eq!(filter(&app), Some("r"), "picker after {keys:?}");
            assert!(!reposted(&app));
            assert_feed_matches_posts(&app);
            assert_eq!(app.status, None);
            assert_feed_matches_posts(&app);
        }

        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!((app.reply.text(), app.mode), ("r", Mode::Reply));
        assert!(!reposted(&app));
        assert_feed_matches_posts(&app);
    }

    // ------------------------------------------------------------ Profiles

    /// Whose profile is open, if any.
    fn viewing(app: &App) -> Option<&'static str> {
        app.profile_page.map(|p| p.name)
    }

    /// Everything a profile must not touch underneath, besides `snapshot`:
    /// the feed, every post's reactions, like state and comments, jobs,
    /// invitations, suggestions, connections, conversations and drafts.
    type World = (
        Snapshot,
        Vec<Card>,
        Vec<(u32, bool, usize)>,
        Vec<(bool, bool)>,
        Vec<&'static str>,
        Vec<bool>,
        u32,
        Vec<(bool, usize)>,
        [TextField; 3],
    );

    fn world(app: &App) -> World {
        (
            snapshot(app),
            app.feed.clone(),
            app.posts
                .iter()
                .map(|p| (p.reactions, p.liked, p.comments.len()))
                .collect(),
            app.jobs.iter().map(|j| (j.saved, j.applied)).collect(),
            app.invitations.iter().map(|p| p.name).collect(),
            app.suggestions.iter().map(|s| s.pending).collect(),
            app.profile.connections,
            app.conversations
                .iter()
                .map(|c| (c.unread, c.messages.len()))
                .collect(),
            [
                app.draft.clone(),
                app.reply.clone(),
                app.comment_draft.clone(),
            ],
        )
    }

    /// Opens My Network with row `row` selected, then the profile with `v`.
    fn network_profile(row: usize) -> App {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        for _ in 0..row {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Char('v'));
        app
    }

    #[test]
    fn v_opens_the_profile_under_the_selection_everywhere() {
        // Home feed: the selected post's author, with nothing else changed.
        let mut app = App::new();
        let before = world(&app);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Maren Holt"));
        assert_eq!(app.profile_page.map(|p| p.post), Some(0));
        assert_eq!(world(&app), before, "the profile sits on top");
        assert!(app.status.is_none());

        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Tobias Ferreira-Lund"));

        // The post view: the post's author, whichever comment is selected.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        let before = world(&app);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Maren Holt"));
        assert_eq!(world(&app), before);
        assert_eq!(app.mode, Mode::PostDetail { comment: 2 });

        // My Network: invitations and suggestions, with `v` and with Enter.
        for (row, name) in [
            (0, "Noor Haddad"),
            (2, "Yuki Tanabe"),
            (3, "Amara Lindgren"),
            (6, "Mateo Achebe"),
        ] {
            let app = network_profile(row);
            assert_eq!(viewing(&app), Some(name), "v on row {row}");
            assert_eq!(app.network_sel, row);

            let mut app = App::new();
            press(&mut app, KeyCode::Char('2'));
            for _ in 0..row {
                press(&mut app, KeyCode::Char('j'));
            }
            let before = world(&app);
            press(&mut app, KeyCode::Enter);
            assert_eq!(viewing(&app), Some(name), "Enter on row {row}");
            assert_eq!(world(&app), before);
        }

        // Messaging: the selected conversation's person, not the open one.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        let before = world(&app);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Ines Okonkwo"));
        assert_eq!(world(&app), before, "nothing is opened or marked read");
        assert_eq!(app.open_chat, Some(0));

        // Jobs and Notifications: nothing.
        for tab in ['3', '5'] {
            let mut app = App::new();
            press(&mut app, KeyCode::Char(tab));
            let before = world(&app);
            press(&mut app, KeyCode::Char('v'));
            assert_eq!(viewing(&app), None, "tab {tab}");
            assert_eq!(world(&app), before, "tab {tab}");
            assert!(app.status.is_none());
        }
    }

    #[test]
    fn v_on_a_repost_card_opens_the_originals_author() {
        let mut app = App::new();
        select_card(&mut app, "Tobias Ferreira-Lund", false);
        press(&mut app, KeyCode::Char('r'));
        select_card(&mut app, "Tobias Ferreira-Lund", true);
        assert_eq!(app.post_sel, 0);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Tobias Ferreira-Lund"));
        assert_eq!(app.post_sel, 0, "still on the repost card");

        // And from the repost's post view.
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Tobias Ferreira-Lund"));

        // A repost of your own post opens your profile.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "hello");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('r'));
        assert!(app.feed[0].repost);
        press(&mut app, KeyCode::Char('k'));
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Jordan Vale"));
        assert_feed_matches_posts(&app);
    }

    #[test]
    fn esc_returns_to_exactly_where_the_profile_was_opened() {
        let mut thread = App::new();
        press(&mut thread, KeyCode::Char('4'));
        press(&mut thread, KeyCode::Char('j'));
        press(&mut thread, KeyCode::Enter);
        press(&mut thread, KeyCode::Esc);
        let mut reposted = App::new();
        press(&mut reposted, KeyCode::Char('r'));
        press(&mut reposted, KeyCode::Char('k'));
        let mut detail = maren_post_view();
        press(&mut detail, KeyCode::Char('j'));
        press(&mut detail, KeyCode::Char('j'));
        let mut feed = App::new();
        for _ in 0..3 {
            press(&mut feed, KeyCode::Char('j'));
        }
        for (name, mut app) in [
            ("feed", feed),
            ("post view", detail),
            ("repost", reposted),
            ("network", network_profile(4)),
            ("messaging", thread),
        ] {
            if app.profile_page.is_some() {
                press(&mut app, KeyCode::Esc);
            }
            let before = world(&app);
            press(&mut app, KeyCode::Char('v'));
            assert!(app.profile_page.is_some(), "{name}");
            // Moving through Activity, an unknown key and a search opened and
            // closed again change nothing underneath.
            press(&mut app, KeyCode::Char('j'));
            press(&mut app, KeyCode::Char('k'));
            press(&mut app, KeyCode::Char('z'));
            press(&mut app, KeyCode::Char('/'));
            type_str(&mut app, "bright");
            press(&mut app, KeyCode::Esc);
            assert!(app.search.is_none(), "{name}");
            assert!(app.profile_page.is_some(), "{name}: back to the profile");
            press(&mut app, KeyCode::Esc);
            assert_eq!(viewing(&app), None, "{name}");
            assert_eq!(world(&app), before, "{name}");
            assert!(!app.should_quit, "{name}");
        }
    }

    #[test]
    fn keys_on_a_profile_dont_act_on_the_tab_below() {
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('v'));
        let before = world(&app);
        for code in [
            KeyCode::Char('l'),
            KeyCode::Char('r'),
            KeyCode::Char('s'),
            KeyCode::Char('p'),
            KeyCode::Char('c'),
            KeyCode::Char('x'),
            KeyCode::Char('a'),
            KeyCode::Char('e'),
            KeyCode::Char('v'),
            KeyCode::Char('J'),
            KeyCode::Char('K'),
            KeyCode::Backspace,
        ] {
            press(&mut app, code);
            assert_eq!(world(&app), before, "{code:?}");
            assert_eq!(viewing(&app), Some("Maren Holt"), "{code:?}");
            assert!(app.send.is_none() && app.status.is_none(), "{code:?}");
        }

        // Ctrl-C quits from a profile, and so does q.
        let mut app = network_profile(0);
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
        let mut app = network_profile(0);
        press(&mut app, KeyCode::Char('q'));
        assert!(app.should_quit);
    }

    #[test]
    fn switching_tabs_closes_the_profile() {
        for (code, tab) in [
            (KeyCode::Char('1'), Tab::Home),
            (KeyCode::Char('2'), Tab::Network),
            (KeyCode::Char('3'), Tab::Jobs),
            (KeyCode::Char('4'), Tab::Messaging),
            (KeyCode::Char('5'), Tab::Notifications),
            (KeyCode::Tab, Tab::Network),
            (KeyCode::BackTab, Tab::Notifications),
        ] {
            let mut app = maren_post_view();
            press(&mut app, KeyCode::Char('j'));
            press(&mut app, KeyCode::Char('v'));
            press(&mut app, code);
            assert_eq!(viewing(&app), None, "{code:?}");
            assert_eq!((app.tab, app.mode), (tab, Mode::Browse), "{code:?}");
            assert_eq!(app.post_sel, 0, "{code:?}");
            // Coming back to Home shows the feed on the same post.
            press(&mut app, KeyCode::Char('1'));
            assert_eq!((app.mode, app.post_sel), (Mode::Browse, 0), "{code:?}");
            assert_eq!(viewing(&app), None, "{code:?}");
        }

        // A search jump from a profile goes where it points and closes it.
        let mut app = network_profile(0);
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "lisbon");
        press(&mut app, KeyCode::Enter);
        assert_eq!((app.tab, app.job_sel), (Tab::Jobs, 0));
        assert_eq!(viewing(&app), None);
    }

    #[test]
    fn accepting_on_a_profile_makes_them_first_degree() {
        let mut app = network_profile(1);
        assert_eq!(viewing(&app), Some("Baptiste Moreau"));
        assert_eq!(app.degree("Baptiste Moreau"), Some(Degree::Second));
        assert_eq!(
            app.profile_action("Baptiste Moreau"),
            Some(ProfileAction::Invitation(1))
        );
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(app.status, Some("Invitation accepted"));
        assert_eq!(app.profile.connections, 313);
        assert_eq!(app.badge(Tab::Network), 2);
        assert_eq!(viewing(&app), Some("Baptiste Moreau"), "the profile stays");
        assert_eq!(app.degree("Baptiste Moreau"), Some(Degree::First));
        assert_eq!(app.profile_action("Baptiste Moreau"), None);

        // No longer an invitation: a, x and c do nothing, without a toast.
        let after = world(&app);
        for ch in ['a', 'x', 'c', 'm'] {
            press(&mut app, KeyCode::Char(ch));
            assert_eq!(world(&app), after, "{ch}");
            assert!(app.status.is_none(), "{ch}");
            assert_eq!(viewing(&app), Some("Baptiste Moreau"));
        }

        // The same as `a` on the row in My Network, selection included.
        press(&mut app, KeyCode::Esc);
        let mut by_hand = App::new();
        press(&mut by_hand, KeyCode::Char('2'));
        press(&mut by_hand, KeyCode::Char('j'));
        press(&mut by_hand, KeyCode::Char('a'));
        by_hand.status = None;
        assert_eq!(world(&app), world(&by_hand));
        assert_eq!(app.network_item(), Some(NetworkItem::Invitation(1)));
        assert_eq!(app.invitations[1].name, "Yuki Tanabe");
    }

    #[test]
    fn ignoring_on_a_profile_leaves_them_second_degree() {
        let mut app = network_profile(0);
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(app.status, Some("Invitation ignored"));
        assert_eq!(app.profile.connections, 312);
        assert_eq!(app.badge(Tab::Network), 2);
        assert_eq!(viewing(&app), Some("Noor Haddad"));
        assert_eq!(app.degree("Noor Haddad"), Some(Degree::Second));
        assert_eq!(app.profile_action("Noor Haddad"), None);
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(app.profile.connections, 312, "nothing left to accept");
        assert!(app.status.is_none());
        press(&mut app, KeyCode::Esc);
        assert_eq!((app.tab, app.mode), (Tab::Network, Mode::Browse));
        assert_eq!(app.network_item(), Some(NetworkItem::Invitation(0)));
        assert_eq!(app.invitations[0].name, "Baptiste Moreau");
    }

    #[test]
    fn removing_the_last_row_from_a_profile_clamps_the_selection() {
        // As on the list: with no suggestions, the cursor moves up.
        for key in ['a', 'x'] {
            let mut app = App::new();
            app.suggestions.clear();
            press(&mut app, KeyCode::Char('2'));
            press(&mut app, KeyCode::Char('j'));
            press(&mut app, KeyCode::Char('j'));
            press(&mut app, KeyCode::Char('v'));
            assert_eq!(viewing(&app), Some("Yuki Tanabe"));
            press(&mut app, KeyCode::Char(key));
            assert_eq!(app.network_sel, 1, "{key}: clamped at once");
            press(&mut app, KeyCode::Esc);
            assert_eq!(
                app.network_item(),
                Some(NetworkItem::Invitation(1)),
                "{key}"
            );

            let mut by_hand = App::new();
            by_hand.suggestions.clear();
            press(&mut by_hand, KeyCode::Char('2'));
            press(&mut by_hand, KeyCode::Char('j'));
            press(&mut by_hand, KeyCode::Char('j'));
            press(&mut by_hand, KeyCode::Char(key));
            by_hand.status = None;
            assert_eq!(world(&app), world(&by_hand), "{key}");
        }

        // With suggestions after it, the first suggestion slides under it.
        let mut app = network_profile(2);
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.network_item(), Some(NetworkItem::Suggestion(0)));
    }

    #[test]
    fn c_on_a_suggestions_profile_connects_and_withdraws() {
        let mut app = network_profile(3);
        assert_eq!(viewing(&app), Some("Amara Lindgren"));
        assert_eq!(
            app.profile_action("Amara Lindgren"),
            Some(ProfileAction::Suggestion(0))
        );
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.status, Some("Invitation sent"));
        assert!(app.suggestions[0].pending);
        assert_eq!(viewing(&app), Some("Amara Lindgren"));
        assert_eq!(app.degree("Amara Lindgren"), Some(Degree::Second));
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.status, Some("Invitation withdrawn"));
        assert!(!app.suggestions[0].pending);

        // a and x on a suggestion, and c on an invitation, do nothing.
        let before = world(&app);
        for ch in ['a', 'x', 'm'] {
            press(&mut app, KeyCode::Char(ch));
            assert_eq!(world(&app), before, "{ch}");
            assert!(app.status.is_none(), "{ch}");
        }
        let mut app = network_profile(0);
        let before = world(&app);
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(world(&app), before);
        assert!(app.status.is_none());

        // On a connection's profile none of them do anything.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('v'));
        let before = world(&app);
        for ch in ['a', 'x', 'c'] {
            press(&mut app, KeyCode::Char(ch));
            assert_eq!(world(&app), before, "{ch}");
            assert!(app.status.is_none(), "{ch}");
            assert_eq!(viewing(&app), Some("Maren Holt"));
        }
    }

    #[test]
    fn m_opens_the_conversation_like_messaging_does() {
        let mut app = App::new();
        app.reply = TextField::from("a stale draft");
        select_card(&mut app, "Ravi Tenorio", false);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(
            app.profile_action("Ravi Tenorio"),
            Some(ProfileAction::Message(1))
        );
        press(&mut app, KeyCode::Char('m'));
        assert_eq!(viewing(&app), None);
        assert_eq!((app.tab, app.mode), (Tab::Messaging, Mode::Reply));
        assert_eq!((app.chat_sel, app.open_chat), (1, Some(1)));
        assert!(!app.conversations[1].unread);
        assert_eq!(app.badge(Tab::Messaging), 1);

        let mut by_hand = App::new();
        by_hand.reply = TextField::from("a stale draft");
        select_card(&mut by_hand, "Ravi Tenorio", false);
        press(&mut by_hand, KeyCode::Char('4'));
        press(&mut by_hand, KeyCode::Char('j'));
        press(&mut by_hand, KeyCode::Enter);
        assert_eq!(world(&app), world(&by_hand));

        // From Messaging, on someone else's thread.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        for _ in 0..3 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Char('v'));
        press(&mut app, KeyCode::Char('m'));
        assert_eq!((app.chat_sel, app.open_chat), (3, Some(3)));
        assert_eq!(app.mode, Mode::Reply);

        // Without a conversation: nothing, the profile stays.
        for (mut app, name) in [
            {
                let mut app = App::new();
                press(&mut app, KeyCode::Char('j'));
                press(&mut app, KeyCode::Char('v'));
                (app, "Tobias Ferreira-Lund")
            },
            (network_profile(5), "Saoirse Kimura"),
        ] {
            assert_eq!(viewing(&app), Some(name));
            let before = world(&app);
            press(&mut app, KeyCode::Char('m'));
            assert_eq!(world(&app), before, "{name}");
            assert_eq!(viewing(&app), Some(name));
            assert!(app.status.is_none());
        }
    }

    #[test]
    fn enter_on_activity_opens_the_post_on_its_own_card() {
        // From Messaging: Ines Okonkwo's post, the third in the feed.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.activity("Ines Okonkwo"), [2]);
        press(&mut app, KeyCode::Enter);
        assert_eq!(viewing(&app), None);
        assert_eq!(app.tab, Tab::Home);
        assert_eq!(app.post_sel, 2);
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });
        assert_eq!(app.selected_post().map(|p| p.author), Some("Ines Okonkwo"));
        // Esc goes back to the feed on her post, as from a comment jump.
        press(&mut app, KeyCode::Esc);
        assert_eq!((app.mode, app.post_sel), (Mode::Browse, 2));

        // The same route as search's comment jump to her post's comment.
        let mut found = App::new();
        press(&mut found, KeyCode::Char('4'));
        press(&mut found, KeyCode::Char('/'));
        type_str(&mut found, "canal view is a strong");
        press(&mut found, KeyCode::Enter);
        let mut via_profile = App::new();
        press(&mut via_profile, KeyCode::Char('4'));
        press(&mut via_profile, KeyCode::Char('j'));
        press(&mut via_profile, KeyCode::Char('j'));
        press(&mut via_profile, KeyCode::Char('v'));
        press(&mut via_profile, KeyCode::Enter);
        assert_eq!(snapshot(&found).0, snapshot(&via_profile).0);
        assert_eq!(found.mode, via_profile.mode);
        assert_eq!(found.post_sel, via_profile.post_sel);

        // From a repost card: its post's own card, not the repost.
        let mut app = App::new();
        select_card(&mut app, "Hana Sorensen", false);
        press(&mut app, KeyCode::Char('r'));
        select_card(&mut app, "Hana Sorensen", true);
        press(&mut app, KeyCode::Char('v'));
        press(&mut app, KeyCode::Enter);
        assert!(!app.feed[app.post_sel].repost);
        assert_eq!(app.post_sel, 6);
        assert_eq!(app.selected_post().map(|p| p.author), Some("Hana Sorensen"));
        assert_eq!(app.mode, Mode::PostDetail { comment: 0 });

        // Enter with no Activity does nothing.
        let mut app = network_profile(0);
        let before = world(&app);
        press(&mut app, KeyCode::Enter);
        assert_eq!(world(&app), before);
        assert_eq!(viewing(&app), Some("Noor Haddad"));
    }

    #[test]
    fn your_own_profile_lists_your_posts_newest_first() {
        let mut app = App::new();
        for text in ["first post", "second post"] {
            press(&mut app, KeyCode::Char('p'));
            type_str(&mut app, text);
            press(&mut app, KeyCode::Enter);
        }
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(viewing(&app), Some("Jordan Vale"));
        assert_eq!(app.activity("Jordan Vale"), [0, 1]);
        assert_eq!(app.posts[0].body, "second post");
        assert_eq!(app.degree("Jordan Vale"), None);
        assert_eq!(app.profile_action("Jordan Vale"), None);
        assert!(app.member("Jordan Vale").is_none());

        // j and k move through Activity and stop at both ends.
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.profile_page.map(|p| p.post), Some(1));
        press(&mut app, KeyCode::Down);
        assert_eq!(app.profile_page.map(|p| p.post), Some(1));
        press(&mut app, KeyCode::Char('k'));
        press(&mut app, KeyCode::Up);
        assert_eq!(app.profile_page.map(|p| p.post), Some(0));
        press(&mut app, KeyCode::Char('j'));

        // a, x, c and m do nothing.
        let before = world(&app);
        for ch in ['a', 'x', 'c', 'm'] {
            press(&mut app, KeyCode::Char(ch));
            assert_eq!(world(&app), before, "{ch}");
            assert!(app.status.is_none(), "{ch}");
        }

        // Enter opens the older post on its own card.
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            (app.post_sel, app.mode),
            (1, Mode::PostDetail { comment: 0 })
        );
        assert_eq!(
            app.selected_post().map(|p| p.body.as_str()),
            Some("first post")
        );

        // Seed authors have one post each; invitations and suggestions none.
        let app = App::new();
        assert_eq!(app.activity("Felix Abernathy"), [6]);
        assert!(app.activity("Noor Haddad").is_empty());
        assert!(app.activity("Jordan Vale").is_empty());
    }

    #[test]
    fn v_and_m_are_typed_in_every_text_box() {
        // The composer.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "vm");
        assert_eq!(app.draft.text(), "vm");
        assert_eq!(viewing(&app), None);

        // The comment box.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "vm");
        assert_eq!(app.comment_draft.text(), "vm");
        assert_eq!(viewing(&app), None);

        // The reply box.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "vm");
        assert_eq!(app.reply.text(), "vm");
        assert_eq!((viewing(&app), app.tab), (None, Tab::Messaging));

        // Search, on every tab, over the post view and over a profile.
        for keys in [
            &[KeyCode::Char('1')][..],
            &[KeyCode::Char('2')],
            &[KeyCode::Char('3')],
            &[KeyCode::Char('4')],
            &[KeyCode::Char('5')],
            &[KeyCode::Enter],
            &[KeyCode::Char('v')],
        ] {
            let mut app = App::new();
            for &k in keys {
                press(&mut app, k);
            }
            let before = world(&app);
            let open = viewing(&app);
            press(&mut app, KeyCode::Char('/'));
            type_str(&mut app, "vm");
            assert_eq!(query(&app), Some("vm"), "{keys:?}");
            assert_eq!(viewing(&app), open, "{keys:?}");
            assert_eq!(world(&app), before, "{keys:?}");
        }

        // The Send picker, from the feed and the post view.
        for mut app in [App::new(), maren_post_view()] {
            press(&mut app, KeyCode::Char('s'));
            let before = world(&app);
            type_str(&mut app, "vm");
            assert_eq!(filter(&app), Some("vm"));
            assert_eq!(viewing(&app), None);
            assert_eq!(world(&app), before);
        }
    }

    /// Every name that appears with a headline, and where.
    fn people_shown(app: &App) -> Vec<(&'static str, &'static str, &'static str)> {
        let mut people = Vec::new();
        for post in &app.posts {
            people.push(("post", post.author, post.headline));
            for c in &post.comments {
                people.push(("comment", c.author, c.headline));
            }
        }
        for p in &app.invitations {
            people.push(("invitation", p.name, p.headline));
        }
        for s in &app.suggestions {
            people.push(("suggestion", s.person.name, s.person.headline));
        }
        for c in &app.conversations {
            people.push(("conversation", c.name, c.headline));
        }
        people
    }

    #[test]
    fn everyone_has_one_profile_with_the_headline_shown_everywhere() {
        let app = App::new();
        let me = app.profile.name;
        for (place, name, headline) in people_shown(&app) {
            if name == me {
                assert_eq!(headline, app.profile.headline, "{place}");
                continue;
            }
            let profiles: Vec<&Member> = app.members.iter().filter(|m| m.name == name).collect();
            assert_eq!(profiles.len(), 1, "{place} {name}: exactly one profile");
            assert_eq!(profiles[0].headline, headline, "{place} {name}");
        }
        // Every profile is someone the app shows, and none is you.
        let shown: Vec<&str> = people_shown(&app).iter().map(|p| p.1).collect();
        for m in &app.members {
            assert!(shown.contains(&m.name), "{}", m.name);
            assert_ne!(m.name, me);
        }
        assert_eq!(app.members.len(), 16);

        for m in &app.members {
            // The current role is the headline, and there's an earlier one.
            let current = &m.experience[0];
            assert_eq!(
                format!("{} at {}", current.title, current.company),
                m.headline,
                "{}",
                m.name
            );
            assert!(current.years.ends_with("present"), "{}", m.name);
            assert!(m.experience.len() >= 2, "{}", m.name);
            assert!(m.location.contains(", "), "{}", m.name);
            assert!(
                m.about.split(". ").count() >= 2,
                "{}: a few sentences",
                m.name
            );
        }

        // Invitations and suggestions agree on mutual counts and aren't
        // connections yet. Everyone who posts or messages you is.
        for p in app
            .invitations
            .iter()
            .chain(app.suggestions.iter().map(|s| &s.person))
        {
            let m = app.member(p.name).expect("has a profile");
            assert_eq!(m.mutual, p.mutual, "{}", p.name);
            assert_eq!(app.degree(p.name), Some(Degree::Second), "{}", p.name);
        }
        for name in app
            .posts
            .iter()
            .map(|p| p.author)
            .chain(app.conversations.iter().map(|c| c.name))
        {
            assert_eq!(app.degree(name), Some(Degree::First), "{name}");
        }
    }

    // ------------------------------------------------------------ Text boxes

    fn edit(field: &mut TextField, code: KeyCode) -> Edit {
        field.on_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn at(field: &TextField) -> (&str, usize) {
        (field.text(), field.cursor())
    }

    #[test]
    fn a_text_field_edits_at_the_cursor() {
        let mut f = TextField::default();
        assert_eq!(at(&f), ("", 0));
        for ch in "helo".chars() {
            assert_eq!(edit(&mut f, KeyCode::Char(ch)), Edit::Changed);
        }
        assert_eq!(at(&f), ("helo", 4));
        assert_eq!(edit(&mut f, KeyCode::Left), Edit::Unchanged);
        assert_eq!(edit(&mut f, KeyCode::Char('l')), Edit::Changed);
        assert_eq!(at(&f), ("hello", 4), "typed before the o, cursor past it");
        edit(&mut f, KeyCode::Home);
        for ch in "Oh, ".chars() {
            edit(&mut f, KeyCode::Char(ch));
        }
        assert_eq!(at(&f), ("Oh, hello", 4));
        edit(&mut f, KeyCode::End);
        assert_eq!(at(&f), ("Oh, hello", 9));

        // Backspace and Delete in the middle.
        let mut f = TextField::from("abcdef");
        assert_eq!(at(&f), ("abcdef", 6), "from() puts the cursor at the end");
        edit(&mut f, KeyCode::Home);
        for _ in 0..3 {
            edit(&mut f, KeyCode::Right);
        }
        assert_eq!(edit(&mut f, KeyCode::Backspace), Edit::Changed);
        assert_eq!(at(&f), ("abdef", 2));
        assert_eq!(edit(&mut f, KeyCode::Delete), Edit::Changed);
        assert_eq!(at(&f), ("abef", 2));

        // Nothing to delete or move to past either end.
        edit(&mut f, KeyCode::Home);
        assert_eq!(edit(&mut f, KeyCode::Backspace), Edit::Unchanged);
        assert_eq!(edit(&mut f, KeyCode::Left), Edit::Unchanged);
        assert_eq!(at(&f), ("abef", 0));
        edit(&mut f, KeyCode::End);
        assert_eq!(edit(&mut f, KeyCode::Delete), Edit::Unchanged);
        assert_eq!(edit(&mut f, KeyCode::Right), Edit::Unchanged);
        assert_eq!(at(&f), ("abef", 4));

        // The owner's keys come back untouched.
        for (code, back) in [
            (KeyCode::Enter, Edit::Enter),
            (KeyCode::Esc, Edit::Esc),
            (KeyCode::Up, Edit::Up),
            (KeyCode::Down, Edit::Down),
            (KeyCode::Tab, Edit::Unchanged),
            (KeyCode::BackTab, Edit::Unchanged),
        ] {
            assert_eq!(edit(&mut f, code), back, "{code:?}");
            assert_eq!(at(&f), ("abef", 4), "{code:?}");
        }

        // Ctrl- and Alt-modified keys do nothing, the arrows included.
        edit(&mut f, KeyCode::Left);
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
            for code in [
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Home,
                KeyCode::End,
                KeyCode::Delete,
                KeyCode::Backspace,
                KeyCode::Char('x'),
                KeyCode::Enter,
                KeyCode::Esc,
                KeyCode::Up,
            ] {
                assert_eq!(
                    f.on_key(KeyEvent::new(code, modifiers)),
                    Edit::Unchanged,
                    "{modifiers:?} {code:?}"
                );
                assert_eq!(at(&f), ("abef", 3), "{modifiers:?} {code:?}");
            }
        }
        // Shift is fine.
        f.on_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT));
        assert_eq!(at(&f), ("abeXf", 4));
    }

    #[test]
    fn a_double_width_character_is_one_step() {
        let mut f = TextField::from("a日b");
        edit(&mut f, KeyCode::Home);
        edit(&mut f, KeyCode::Right);
        assert_eq!(at(&f), ("a日b", 1));
        edit(&mut f, KeyCode::Right);
        assert_eq!(at(&f), ("a日b", 4), "over 日 in one step");
        edit(&mut f, KeyCode::Left);
        assert_eq!(at(&f), ("a日b", 1), "and back");
        edit(&mut f, KeyCode::Delete);
        assert_eq!(at(&f), ("ab", 1), "日 deleted whole");
        edit(&mut f, KeyCode::Char('日'));
        assert_eq!(at(&f), ("a日b", 4));
        edit(&mut f, KeyCode::Backspace);
        assert_eq!(at(&f), ("ab", 1), "日 deleted whole");

        // Every key from every cursor position over characters of 1 to 4
        // bytes keeps the cursor on a `char` boundary, without panicking.
        let text = "日a本é語b\u{1D11E}";
        let chars = text.chars().count();
        for start in 0..=chars {
            for code in [
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Home,
                KeyCode::End,
                KeyCode::Backspace,
                KeyCode::Delete,
                KeyCode::Char('x'),
                KeyCode::Char('字'),
            ] {
                let mut f = TextField::from(text);
                edit(&mut f, KeyCode::Home);
                for _ in 0..start {
                    edit(&mut f, KeyCode::Right);
                }
                assert_eq!(f.text()[..f.cursor()].chars().count(), start);
                edit(&mut f, code);
                assert!(
                    f.cursor() <= f.text().len() && f.text().is_char_boundary(f.cursor()),
                    "{code:?} from {start}: {} in {:?}",
                    f.cursor(),
                    f.text()
                );
            }
        }
    }

    fn draft_of(app: &App) -> &TextField {
        &app.draft
    }

    fn comment_draft_of(app: &App) -> &TextField {
        &app.comment_draft
    }

    fn reply_of(app: &App) -> &TextField {
        &app.reply
    }

    fn query_of(app: &App) -> &TextField {
        &app.search.as_ref().expect("search is open").query
    }

    fn filter_of(app: &App) -> &TextField {
        &app.send.as_ref().expect("the picker is open").query
    }

    /// The five text boxes, each just opened on the seed: its name, the app,
    /// how to read its field, and whether `↑`/`↓` move a list under it.
    type TextBox = (&'static str, App, fn(&App) -> &TextField, bool);

    fn text_boxes() -> Vec<TextBox> {
        let open = |keys: &[KeyCode]| {
            let mut app = App::new();
            for &code in keys {
                press(&mut app, code);
            }
            app
        };
        vec![
            ("composer", open(&[KeyCode::Char('p')]), draft_of, false),
            (
                "comment box",
                open(&[KeyCode::Enter, KeyCode::Char('c')]),
                comment_draft_of,
                false,
            ),
            (
                "reply box",
                open(&[KeyCode::Char('4'), KeyCode::Enter]),
                reply_of,
                false,
            ),
            ("search", open(&[KeyCode::Char('/')]), query_of, true),
            ("send picker", open(&[KeyCode::Char('s')]), filter_of, true),
        ]
    }

    #[test]
    fn every_text_box_moves_the_cursor_and_edits_in_the_middle() {
        for (name, mut app, field, _) in text_boxes() {
            let before = snapshot(&app);
            let mode = app.mode;
            let at = |app: &App| (field(app).text().to_owned(), field(app).cursor());
            let is = |text: &str, cursor: usize| (text.to_owned(), cursor);
            assert_eq!(at(&app), is("", 0), "{name} opens empty");

            // `←` then typing inserts in the middle.
            type_str(&mut app, "helo");
            press(&mut app, KeyCode::Left);
            type_str(&mut app, "l");
            assert_eq!(at(&app), is("hello", 4), "{name}");
            // `Home` then typing inserts at the start, `End` goes back.
            press(&mut app, KeyCode::Home);
            type_str(&mut app, "Oh, ");
            assert_eq!(at(&app), is("Oh, hello", 4), "{name}");
            press(&mut app, KeyCode::End);
            assert_eq!(at(&app), is("Oh, hello", 9), "{name}");

            // `Backspace` and `Delete` in the middle.
            press(&mut app, KeyCode::Home);
            for _ in 0..3 {
                press(&mut app, KeyCode::Right);
            }
            press(&mut app, KeyCode::Backspace);
            assert_eq!(at(&app), is("Oh hello", 2), "{name}: the comma");
            press(&mut app, KeyCode::Delete);
            assert_eq!(at(&app), is("Ohhello", 2), "{name}: the space");

            // At the start, `Backspace` and `←` change nothing; at the end,
            // `Delete` and `→` don't.
            press(&mut app, KeyCode::Home);
            press(&mut app, KeyCode::Backspace);
            press(&mut app, KeyCode::Left);
            assert_eq!(at(&app), is("Ohhello", 0), "{name}");
            press(&mut app, KeyCode::End);
            press(&mut app, KeyCode::Delete);
            press(&mut app, KeyCode::Right);
            assert_eq!(at(&app), is("Ohhello", 7), "{name}");

            // `日` is one step, and deleted whole either way.
            press(&mut app, KeyCode::Home);
            type_str(&mut app, "日");
            assert_eq!(at(&app), is("日Ohhello", 3), "{name}");
            press(&mut app, KeyCode::Left);
            assert_eq!(at(&app), is("日Ohhello", 0), "{name}");
            press(&mut app, KeyCode::Right);
            assert_eq!(at(&app), is("日Ohhello", 3), "{name}");
            press(&mut app, KeyCode::Backspace);
            assert_eq!(at(&app), is("Ohhello", 0), "{name}");
            type_str(&mut app, "日");
            press(&mut app, KeyCode::Home);
            press(&mut app, KeyCode::Delete);
            assert_eq!(at(&app), is("Ohhello", 0), "{name}");

            // Ctrl- and Alt-modified editing keys, `Tab` and `BackTab` do
            // nothing.
            press(&mut app, KeyCode::Right);
            for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
                for code in [
                    KeyCode::Left,
                    KeyCode::Right,
                    KeyCode::Home,
                    KeyCode::End,
                    KeyCode::Delete,
                    KeyCode::Backspace,
                ] {
                    app.on_key(KeyEvent::new(code, modifiers));
                }
            }
            press(&mut app, KeyCode::Tab);
            press(&mut app, KeyCode::BackTab);
            assert_eq!(at(&app), is("Ohhello", 1), "{name}");

            assert_eq!(snapshot(&app), before, "{name}: nothing else moved");
            assert_eq!(app.mode, mode, "{name}");
            assert!(app.status.is_none() && !app.should_quit, "{name}");
        }
    }

    #[test]
    fn up_and_down_do_nothing_in_the_composer_comment_and_reply_boxes() {
        for (name, mut app, field, lists) in text_boxes() {
            if lists {
                continue;
            }
            type_str(&mut app, "abc");
            press(&mut app, KeyCode::Left);
            let before = (snapshot(&app), field(&app).clone());
            press(&mut app, KeyCode::Up);
            press(&mut app, KeyCode::Down);
            assert_eq!((snapshot(&app), field(&app).clone()), before, "{name}");
        }
    }

    #[test]
    fn edits_in_the_middle_are_what_gets_submitted() {
        // The composer publishes the edited draft.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "Helo world");
        for _ in 0..7 {
            press(&mut app, KeyCode::Left);
        }
        type_str(&mut app, "l");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.posts[0].body, "Hello world");
        assert_eq!(app.posts[0].author, "Jordan Vale");
        assert_eq!(app.status, Some("Post published"));

        // The comment box posts the edited comment.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "Grat post ");
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);
        type_str(&mut app, "e");
        press(&mut app, KeyCode::Enter);
        let mine = app.posts[0].comments.last().expect("just posted");
        assert_eq!(mine.body, "Great post", "edited, then trimmed");
        assert_eq!(app.status, Some("Comment posted"));

        // The reply box sends the edited reply.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "See yo there");
        for _ in 0..6 {
            press(&mut app, KeyCode::Left);
        }
        type_str(&mut app, "u");
        press(&mut app, KeyCode::Enter);
        let last = app.conversations[0].messages.last().expect("just sent");
        assert!(last.from_me);
        assert_eq!(last.body, "See you there");

        // Search opens the result of the edited query.
        let mut typed = App::new();
        press(&mut typed, KeyCode::Char('/'));
        type_str(&mut typed, "lisbon");
        let mut edited = App::new();
        press(&mut edited, KeyCode::Char('/'));
        type_str(&mut edited, "lisbn");
        press(&mut edited, KeyCode::Left);
        type_str(&mut edited, "o");
        assert_eq!(query(&edited), Some("lisbon"));
        press(&mut typed, KeyCode::Enter);
        press(&mut edited, KeyCode::Enter);
        assert_eq!(edited.tab, Tab::Jobs);
        assert_eq!(snapshot(&edited), snapshot(&typed));

        // The Send picker sends to the recipient the edited filter selects.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "flix");
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Right);
        type_str(&mut app, "e");
        assert_eq!(filter(&app), Some("felix"));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Post sent"));
        let felix = app
            .conversations
            .iter()
            .find(|c| c.name == "Felix Abernathy")
            .expect("Felix is in the seed");
        let last = felix.messages.last().expect("just sent");
        assert!(last.body.starts_with("Shared a post by Maren Holt: "));
    }

    #[test]
    fn only_a_real_change_to_the_query_selects_the_first_result_again() {
        let boxes = [
            ('/', "bright", selected_hit as fn(&App) -> usize),
            ('s', "o", selected_recipient),
        ];
        for (open, text, selected) in boxes {
            let mut app = App::new();
            press(&mut app, KeyCode::Char(open));
            type_str(&mut app, text);
            press(&mut app, KeyCode::Down);
            press(&mut app, KeyCode::Down);
            assert_eq!(selected(&app), 2, "{text}");

            // Moving the cursor, and deleting nothing, keep the selection.
            for code in [
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Home,
                KeyCode::Backspace,
                KeyCode::Left,
                KeyCode::End,
                KeyCode::Delete,
                KeyCode::Right,
            ] {
                press(&mut app, code);
                assert_eq!(selected(&app), 2, "{text}: {code:?}");
            }

            // An edit in the middle selects the first result again.
            press(&mut app, KeyCode::Left);
            type_str(&mut app, "x");
            assert_eq!(selected(&app), 0, "{text}: typed in the middle");
            press(&mut app, KeyCode::Backspace);
            assert_eq!(selected(&app), 0);
            press(&mut app, KeyCode::Down);
            press(&mut app, KeyCode::Down);
            assert_eq!(selected(&app), 2);
            press(&mut app, KeyCode::Home);
            press(&mut app, KeyCode::Delete);
            assert_eq!(selected(&app), 0, "{text}: deleted at the start");
            press(&mut app, KeyCode::Down);
            press(&mut app, KeyCode::Backspace);
            assert_eq!(selected(&app), 1, "{text}: nothing before the cursor");
        }
    }

    #[test]
    fn every_box_opens_empty_with_the_cursor_at_0() {
        let fresh = TextField::default();
        let half = |app: &mut App| {
            type_str(app, "half");
            press(app, KeyCode::Left);
            press(app, KeyCode::Left);
        };

        // The composer, after Esc, on reopening and after publishing.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        half(&mut app);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.draft, fresh);
        press(&mut app, KeyCode::Char('p'));
        assert_eq!(app.draft, fresh);
        half(&mut app);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Post published"));
        assert_eq!(app.draft, fresh);

        // The comment box.
        let mut app = maren_post_view();
        press(&mut app, KeyCode::Char('c'));
        half(&mut app);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.comment_draft, fresh);
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(app.comment_draft, fresh);
        half(&mut app);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.status, Some("Comment posted"));
        assert_eq!(app.comment_draft, fresh);

        // The reply box, also when a conversation opens over a stale draft,
        // from Messaging and from search.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        half(&mut app);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.reply, fresh);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.reply, fresh);
        half(&mut app);
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.conversations[0]
                .messages
                .last()
                .map(|m| m.body.as_str()),
            Some("half")
        );
        assert_eq!(app.reply, fresh);
        press(&mut app, KeyCode::Esc);
        app.reply = TextField::from("a stale draft");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.reply, fresh);
        press(&mut app, KeyCode::Esc);
        app.reply = TextField::from("a stale draft");
        press(&mut app, KeyCode::Char('/'));
        // Ravi Tenorio's conversation is `agenda`'s third result.
        type_str(&mut app, "agenda");
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!((app.mode, app.open_chat), (Mode::Reply, Some(1)));
        assert_eq!(app.reply, fresh);

        // Search and the Send picker, after Esc and after Enter.
        for open in ['/', 's'] {
            let picker = |app: &App| {
                if open == '/' {
                    app.search.as_ref().map(|s| s.query.clone())
                } else {
                    app.send.as_ref().map(|p| p.query.clone())
                }
            };
            let mut app = App::new();
            press(&mut app, KeyCode::Char(open));
            assert_eq!(picker(&app), Some(fresh.clone()));
            type_str(&mut app, "maren");
            press(&mut app, KeyCode::Left);
            press(&mut app, KeyCode::Esc);
            assert_eq!(picker(&app), None);
            press(&mut app, KeyCode::Char(open));
            assert_eq!(picker(&app), Some(fresh.clone()), "{open}: reopened");
            type_str(&mut app, "maren");
            press(&mut app, KeyCode::Left);
            press(&mut app, KeyCode::Enter);
            assert_eq!(picker(&app), None, "{open}: Enter closes it");
            press(&mut app, KeyCode::Char('1'));
            press(&mut app, KeyCode::Esc);
            press(&mut app, KeyCode::Char(open));
            assert_eq!(picker(&app), Some(fresh.clone()), "{open}: after Enter");
        }
    }

    #[test]
    fn left_right_home_end_and_delete_do_nothing_outside_the_text_boxes() {
        let at = |keys: &[KeyCode]| {
            let mut app = App::new();
            for &code in keys {
                press(&mut app, code);
            }
            app
        };
        let states = [
            ("home", at(&[KeyCode::Char('j')])),
            (
                "post view",
                at(&[KeyCode::Enter, KeyCode::Char('j'), KeyCode::Char('j')]),
            ),
            ("network", at(&[KeyCode::Char('2'), KeyCode::Char('j')])),
            ("jobs", at(&[KeyCode::Char('3'), KeyCode::Char('j')])),
            ("messaging", at(&[KeyCode::Char('4'), KeyCode::Char('j')])),
            (
                "messaging, thread open",
                at(&[KeyCode::Char('4'), KeyCode::Enter, KeyCode::Esc]),
            ),
            ("notifications", at(&[KeyCode::Char('5')])),
            ("profile", at(&[KeyCode::Char('v')])),
            ("own profile", {
                let mut app = at(&[KeyCode::Char('p')]);
                type_str(&mut app, "hello");
                press(&mut app, KeyCode::Enter);
                press(&mut app, KeyCode::Char('v'));
                app
            }),
        ];
        for (name, mut app) in states {
            app.status = None;
            let before = (world(&app), app.profile_page);
            for code in [
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Home,
                KeyCode::End,
                KeyCode::Delete,
            ] {
                press(&mut app, code);
                assert_eq!((world(&app), app.profile_page), before, "{name}: {code:?}");
                assert!(app.search.is_none() && app.send.is_none(), "{name}");
                assert!(app.status.is_none() && !app.should_quit, "{name}");
            }
        }
    }

    // ------------------------------------------------------------ Job filters

    /// A fresh app on the Jobs tab.
    fn jobs_tab() -> App {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        app
    }

    fn shown_titles(app: &App) -> Vec<&'static str> {
        app.shown_jobs()
            .iter()
            .map(|&i| app.jobs[i].title)
            .collect()
    }

    fn selected_title(app: &App) -> Option<&'static str> {
        app.selected_job().map(|i| app.jobs[i].title)
    }

    // The seed's jobs, by index.
    const RUST: &str = "Senior Rust Engineer";
    const DESIGNER: &str = "Terminal UI Designer";
    const SRE: &str = "Site Reliability Engineer";
    const ADVOCATE: &str = "Developer Advocate";
    const DATA: &str = "Data Engineer";
    const MANAGER: &str = "Engineering Manager, Build Tools";

    #[test]
    fn f_cycles_through_the_filters_and_wraps() {
        let mut app = jobs_tab();
        assert_eq!(app.job_filter, JobFilter::All);
        assert_eq!(
            shown_titles(&app),
            [RUST, DESIGNER, SRE, ADVOCATE, DATA, MANAGER]
        );
        for (filter, titles) in [
            (JobFilter::Remote, &[RUST, ADVOCATE][..]),
            (JobFilter::Hybrid, &[DESIGNER, MANAGER]),
            (JobFilter::OnSite, &[SRE, DATA]),
            (JobFilter::Saved, &[RUST]),
            (
                JobFilter::All,
                &[RUST, DESIGNER, SRE, ADVOCATE, DATA, MANAGER],
            ),
        ] {
            press(&mut app, KeyCode::Char('f'));
            assert_eq!(app.job_filter, filter);
            assert_eq!(shown_titles(&app), titles, "{filter:?}");
            assert_eq!(app.status, None, "{filter:?}: no toast");
            assert_eq!((app.tab, app.mode), (Tab::Jobs, Mode::Browse));
        }
        assert_eq!(
            JobFilter::ALL.map(JobFilter::label),
            ["All", "Remote", "Hybrid", "On-site", "Saved"]
        );
    }

    #[test]
    fn the_filter_keeps_the_selected_job_or_moves_to_the_first_match() {
        // Developer Advocate, scrolled two lines.
        let mut app = jobs_overflowing(5);
        for _ in 0..3 {
            press(&mut app, KeyCode::Char('j'));
        }
        shift(&mut app, 'J');
        shift(&mut app, 'J');
        assert_eq!((selected_title(&app), app.job_scroll), (Some(ADVOCATE), 2));

        let steps = [
            // Remote still shows him: same job, same scroll.
            (JobFilter::Remote, ADVOCATE, 2),
            // Hybrid doesn't: its first job, from the top.
            (JobFilter::Hybrid, DESIGNER, 0),
            (JobFilter::OnSite, SRE, 0),
            (JobFilter::Saved, RUST, 0),
            // All shows the Senior Rust Engineer too: kept, scroll and all.
            (JobFilter::All, RUST, 1),
        ];
        for (filter, title, scroll) in steps {
            press(&mut app, KeyCode::Char('f'));
            assert_eq!(app.job_filter, filter);
            assert_eq!(selected_title(&app), Some(title), "{filter:?}");
            assert_eq!(app.job_sel, app.selected_job().expect("a job"));
            assert_eq!(app.job_scroll, scroll, "{filter:?}");
            // Scroll one line, to see whether the next `f` keeps it.
            if filter == JobFilter::Saved {
                shift(&mut app, 'J');
            }
        }
    }

    #[test]
    fn j_and_k_stop_at_both_ends_of_a_filtered_list() {
        let mut app = jobs_overflowing(5);
        type_str(&mut app, "ff");
        assert_eq!(selected_title(&app), Some(DESIGNER));
        shift(&mut app, 'J');
        for code in [KeyCode::Char('k'), KeyCode::Up] {
            press(&mut app, code);
            assert_eq!(selected_title(&app), Some(DESIGNER), "{code:?} at the top");
            assert_eq!(app.job_scroll, 1, "{code:?} keeps the scroll");
        }
        // Down skips the jobs Hybrid hides.
        press(&mut app, KeyCode::Char('j'));
        assert_eq!((selected_title(&app), app.job_scroll), (Some(MANAGER), 0));
        shift(&mut app, 'J');
        for code in [KeyCode::Char('j'), KeyCode::Down] {
            press(&mut app, code);
            assert_eq!(selected_title(&app), Some(MANAGER), "{code:?} at the end");
            assert_eq!(app.job_scroll, 1, "{code:?} keeps the scroll");
        }
        press(&mut app, KeyCode::Up);
        assert_eq!((selected_title(&app), app.job_scroll), (Some(DESIGNER), 0));

        // One job: neither moves.
        let mut app = jobs_tab();
        type_str(&mut app, "ffff");
        for code in [KeyCode::Char('j'), KeyCode::Char('k')] {
            press(&mut app, code);
            assert_eq!(selected_title(&app), Some(RUST), "{code:?}");
        }
    }

    #[test]
    fn an_empty_saved_list_ignores_every_job_key() {
        let mut app = jobs_overflowing(5);
        press(&mut app, KeyCode::Char('s'));
        type_str(&mut app, "ffff");
        assert_eq!(app.job_filter, JobFilter::Saved);
        assert!(app.shown_jobs().is_empty());
        assert_eq!(app.selected_job(), None);
        assert_eq!(app.saved_jobs(), 0);
        app.status = None;

        let before = (unscrolled(&app), app.job_scroll, app.job_filter);
        for ch in "sejkJK".chars() {
            if ch.is_uppercase() {
                shift(&mut app, ch);
            } else {
                press(&mut app, KeyCode::Char(ch));
            }
            assert_eq!(
                (unscrolled(&app), app.job_scroll, app.job_filter),
                before,
                "{ch}"
            );
        }
        for code in [
            KeyCode::Down,
            KeyCode::Up,
            KeyCode::Enter,
            KeyCode::Char('v'),
        ] {
            press(&mut app, code);
            assert_eq!(
                (unscrolled(&app), app.job_scroll, app.job_filter),
                before,
                "{code:?}"
            );
        }

        // `f` still works.
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(app.job_filter, JobFilter::All);
        assert_eq!(selected_title(&app), Some(RUST));
    }

    #[test]
    fn leaving_an_empty_list_selects_the_first_match() {
        // Data Engineer saved too, then both unsaved under Saved, the Senior
        // Rust Engineer first, so `job_sel` is left on the Data Engineer.
        let mut app = jobs_tab();
        type_str(&mut app, "jjjjs");
        type_str(&mut app, "ffff");
        assert_eq!(shown_titles(&app), [RUST, DATA]);
        // Remote moved the selection off him, so Saved starts on its first.
        assert_eq!(selected_title(&app), Some(RUST));
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(selected_title(&app), Some(DATA));
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.selected_job(), None);
        // Nothing was selected, so `All` selects its first job.
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(
            (app.job_filter, selected_title(&app)),
            (JobFilter::All, Some(RUST))
        );
    }

    /// Jobs on Saved with three saved jobs, the Senior Rust Engineer (the
    /// seed's), the Site Reliability Engineer and the Data Engineer, the last
    /// selected and scrolled a line.
    fn three_saved() -> App {
        let mut app = jobs_overflowing(5);
        type_str(&mut app, "jjsjjs");
        type_str(&mut app, "ffff");
        assert_eq!(app.job_filter, JobFilter::Saved);
        assert_eq!(shown_titles(&app), [RUST, SRE, DATA]);
        // On-site left the Site Reliability Engineer selected, and Saved
        // keeps him.
        assert_eq!(selected_title(&app), Some(SRE));
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(selected_title(&app), Some(DATA));
        shift(&mut app, 'J');
        app
    }

    #[test]
    fn unsaving_under_saved_moves_to_the_next_job() {
        // The first row: the next job slides under the cursor.
        let mut app = three_saved();
        type_str(&mut app, "kk");
        shift(&mut app, 'J');
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.status, Some("Job unsaved"));
        assert_eq!(shown_titles(&app), [SRE, DATA]);
        assert_eq!((selected_title(&app), app.job_scroll), (Some(SRE), 0));
        assert_eq!(app.saved_jobs(), 2);

        // A middle row: the same.
        let mut app = three_saved();
        press(&mut app, KeyCode::Char('k'));
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.status, Some("Job unsaved"));
        assert_eq!(shown_titles(&app), [RUST, DATA]);
        assert_eq!(selected_title(&app), Some(DATA));

        // The last row: the cursor moves up to the new last job.
        let mut app = three_saved();
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.status, Some("Job unsaved"));
        assert_eq!(shown_titles(&app), [RUST, SRE]);
        assert_eq!((selected_title(&app), app.job_scroll), (Some(SRE), 0));

        // Down to none, each with its toast.
        for left in [1, 0] {
            press(&mut app, KeyCode::Char('s'));
            assert_eq!(app.status, Some("Job unsaved"));
            assert_eq!(app.shown_jobs().len(), left);
        }
        assert_eq!((app.selected_job(), app.saved_jobs()), (None, 0));
        // The jobs are still there for the other filters.
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(app.shown_jobs().len(), 6);
    }

    #[test]
    fn saving_under_other_filters_keeps_the_selection() {
        let mut app = jobs_overflowing(5);
        press(&mut app, KeyCode::Char('j'));
        shift(&mut app, 'J');
        for want in ["Job saved", "Job unsaved"] {
            press(&mut app, KeyCode::Char('s'));
            assert_eq!(app.status, Some(want));
            assert_eq!((selected_title(&app), app.job_scroll), (Some(DESIGNER), 1));
            assert_eq!(app.shown_jobs().len(), 6);
        }

        // Unsaving under Remote leaves the job in the Remote list.
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(selected_title(&app), Some(RUST));
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.status, Some("Job unsaved"));
        assert_eq!(selected_title(&app), Some(RUST));
        assert_eq!(shown_titles(&app), [RUST, ADVOCATE]);
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.status, Some("Job saved"));
        assert_eq!(selected_title(&app), Some(ADVOCATE));
        assert_eq!(app.saved_jobs(), 1);

        // Saved now lists him alone, and keeps him selected.
        type_str(&mut app, "fff");
        assert_eq!(shown_titles(&app), [ADVOCATE]);
        assert_eq!(selected_title(&app), Some(ADVOCATE));
    }

    /// Opens search, types `query` and presses Enter on the job result
    /// titled `title`.
    fn jump_to_job(app: &mut App, query: &str, title: &str) {
        let down = app
            .search_hits(query)
            .iter()
            .position(|&hit| matches!(hit, SearchHit::Job(i) if app.jobs[i].title == title))
            .unwrap_or_else(|| panic!("{query:?} finds {title}"));
        jump(app, query, down);
    }

    #[test]
    fn a_search_jump_shows_the_job_even_when_the_filter_hides_it() {
        // Hidden by Hybrid: back to All.
        let mut app = jobs_overflowing(5);
        type_str(&mut app, "ff");
        shift(&mut app, 'J');
        jump_to_job(&mut app, "lisbon", RUST);
        assert_eq!(app.tab, Tab::Jobs);
        assert_eq!(app.job_filter, JobFilter::All);
        assert_eq!((selected_title(&app), app.job_scroll), (Some(RUST), 0));

        // Shown by Remote: the filter stays.
        let mut app = jobs_overflowing(5);
        press(&mut app, KeyCode::Char('f'));
        shift(&mut app, 'J');
        jump_to_job(&mut app, "brightkettle", ADVOCATE);
        assert_eq!(app.job_filter, JobFilter::Remote);
        assert_eq!((selected_title(&app), app.job_scroll), (Some(ADVOCATE), 0));

        // From another tab, under Saved, to a job that isn't saved.
        let mut app = jobs_tab();
        type_str(&mut app, "ffff");
        press(&mut app, KeyCode::Char('1'));
        jump_to_job(&mut app, "robotics", DATA);
        assert_eq!((app.tab, app.job_filter), (Tab::Jobs, JobFilter::All));
        assert_eq!(selected_title(&app), Some(DATA));

        // From an empty Saved list.
        let mut app = jobs_tab();
        type_str(&mut app, "sffff");
        assert!(app.shown_jobs().is_empty());
        jump_to_job(&mut app, "lisbon", RUST);
        assert_eq!(app.job_filter, JobFilter::All);
        assert_eq!(selected_title(&app), Some(RUST));
    }

    #[test]
    fn search_finds_every_job_whatever_the_filter() {
        let jobs = |app: &App| -> Vec<&'static str> {
            app.search_hits("engineer")
                .into_iter()
                .filter_map(|hit| match hit {
                    SearchHit::Job(i) => Some(app.jobs[i].title),
                    _ => None,
                })
                .collect()
        };
        let mut app = jobs_tab();
        let all = jobs(&app);
        assert_eq!(all, [RUST, SRE, DATA, MANAGER]);
        for _ in 0..4 {
            press(&mut app, KeyCode::Char('f'));
            assert_eq!(jobs(&app), all, "{:?}", app.job_filter);
        }
    }

    #[test]
    fn the_filter_is_kept_like_a_selection() {
        let mut app = jobs_overflowing(5);
        type_str(&mut app, "fffj");
        let kept = (JobFilter::OnSite, 4);
        assert_eq!(selected_title(&app), Some(DATA));
        let check = |app: &App, what: &str| {
            assert_eq!((app.job_filter, app.job_sel), kept, "{what}");
        };

        type_str(&mut app, "13");
        check(&app, "1 then 3");
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::BackTab);
        check(&app, "Tab then BackTab");
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "rust");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.tab, Tab::Jobs);
        check(&app, "/ then Esc");

        // A profile, opened on Home, closed with Esc and with a tab switch.
        type_str(&mut app, "1v");
        assert!(app.profile_page.is_some());
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('3'));
        check(&app, "a profile then Esc");
        type_str(&mut app, "1v3");
        assert!(app.profile_page.is_none());
        check(&app, "a profile then 3");

        type_str(&mut app, "ssee");
        check(&app, "s and e");
        shift(&mut app, 'J');
        shift(&mut app, 'K');
        check(&app, "J and K");

        // A search jump anywhere but to a job leaves it alone.
        jump(&mut app, "typo", 0);
        assert_eq!(app.tab, Tab::Home);
        press(&mut app, KeyCode::Char('3'));
        check(&app, "a jump to a post");
    }

    #[test]
    fn f_does_nothing_off_jobs() {
        let routes: [&[KeyCode]; 8] = [
            &[KeyCode::Char('1')],
            &[KeyCode::Char('1'), KeyCode::Enter],
            &[KeyCode::Char('1'), KeyCode::Char('v')],
            &[KeyCode::Char('2')],
            &[KeyCode::Char('2'), KeyCode::Char('v')],
            &[KeyCode::Char('4')],
            &[KeyCode::Char('4'), KeyCode::Enter, KeyCode::Esc],
            &[KeyCode::Char('5')],
        ];
        for route in routes {
            // With the filter on Remote, to see it isn't stepped.
            let mut app = jobs_tab();
            press(&mut app, KeyCode::Char('f'));
            for &key in route {
                press(&mut app, key);
            }
            let before = (world(&app), app.profile_page, app.job_filter);
            press(&mut app, KeyCode::Char('f'));
            assert_eq!(
                (world(&app), app.profile_page, app.job_filter),
                before,
                "{route:?}"
            );
            assert_eq!(app.status, None, "{route:?}: no toast");
            assert_eq!(app.job_filter, JobFilter::Remote, "{route:?}");
        }
    }

    #[test]
    fn f_is_typed_in_every_text_box() {
        // Each box opened with the filter on Remote.
        let remote = || {
            let mut app = jobs_tab();
            press(&mut app, KeyCode::Char('f'));
            app
        };

        for tab in ['1', '2', '3', '4', '5'] {
            let mut app = remote();
            press(&mut app, KeyCode::Char(tab));
            press(&mut app, KeyCode::Char('/'));
            let before = snapshot(&app);
            type_str(&mut app, "ff");
            assert_eq!(query(&app), Some("ff"), "search on {tab}");
            assert_eq!(snapshot(&app), before, "search on {tab}");
            assert_eq!(app.job_filter, JobFilter::Remote, "search on {tab}");
        }

        let mut app = remote();
        type_str(&mut app, "1p");
        type_str(&mut app, "ff");
        assert_eq!((app.draft.text(), app.mode), ("ff", Mode::Composer));
        assert_eq!(app.job_filter, JobFilter::Remote, "composer");

        let mut app = remote();
        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, "ff");
        assert_eq!(app.comment_draft.text(), "ff");
        assert_eq!(app.job_filter, JobFilter::Remote, "comment box");

        for keys in [&[][..], &[KeyCode::Enter]] {
            let mut app = remote();
            press(&mut app, KeyCode::Char('1'));
            for &key in keys {
                press(&mut app, key);
            }
            press(&mut app, KeyCode::Char('s'));
            type_str(&mut app, "ff");
            assert_eq!(filter(&app), Some("ff"), "picker after {keys:?}");
            assert_eq!(app.job_filter, JobFilter::Remote, "picker after {keys:?}");
        }

        let mut app = remote();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "ff");
        assert_eq!((app.reply.text(), app.mode), ("ff", Mode::Reply));
        assert_eq!(app.job_filter, JobFilter::Remote, "reply box");
    }
}
