use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;

use crate::data;
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
    /// Home: the "Start a post" composer overlay. Swallows printable keys.
    Composer,
    /// Messaging: the reply box under the open thread. Swallows printable keys.
    Reply,
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
        match self.mode {
            Mode::Composer => self.on_composer_key(key),
            Mode::Reply => self.on_reply_key(key),
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
            _ => match self.tab {
                Tab::Home => self.on_home_key(code),
                Tab::Network => self.on_network_key(code),
                Tab::Jobs => self.on_jobs_key(code),
                Tab::Messaging => self.on_messaging_key(code),
                Tab::Notifications => {}
            },
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
            _ => {}
        }
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
}
