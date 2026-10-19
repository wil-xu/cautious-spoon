# workedin — a LinkedIn-style TUI with fictional coworkers

A terminal app that copies the linkedin.com experience: a feed of posts from
coworkers, invitations and suggested connections, a job board with Easy Apply,
messaging, and notifications. Every person, company, post and job is invented.

**Status:** implemented. This file describes the code as built. When the two
disagree, fix whichever one is wrong. Don't let them drift apart.

## Goals

- At a glance it looks like LinkedIn: a top bar with the `in` logo, a search
  box and five tabs with red count badges, a three-column Home (profile card,
  feed, news), and the familiar My Network, Jobs, Messaging and Notifications
  pages.
- It responds the way LinkedIn does: liking posts, opening a post with its
  comments, writing a post, accepting or ignoring invitations, sending
  connection requests, saving jobs and applying, reading and replying to
  messages, and clearing notifications. `/` searches people, jobs and posts
  and jumps to the result.
- One self-contained binary. No network, no files written at runtime and no
  real data: all content is hard-coded, fictional and safe for work.

## Non-goals

- Real LinkedIn data or accounts (would need auth and APIs, and raises privacy
  concerns).
- Persisting state between runs. Likes, posts, connections, saved jobs and
  messages reset on restart.
- Mouse support.

## Build & run

A plain Cargo crate. The devserver had no Rust toolchain, so one was installed
with the official `rustup-init` (stable 1.99.0, `minimal` profile plus
`rustfmt` and `clippy`) into the default `~/.rustup` and `~/.cargo`, with
`--no-modify-path`. Put `~/.cargo/bin` on `PATH` first:

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo run
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

- **Dependencies:** just `ratatui` (0.30, locked to 0.30.2 in `Cargo.lock`). It
  re-exports `crossterm` as `ratatui::crossterm`, which keeps the backend and
  the event types on the same version.
- **Edition:** 2024, which the event loop and two handlers need for
  let-chains.
- **Profile:** `[profile.dev.package."*"] opt-level = 2` optimizes ratatui in
  dev and test builds. Without it the every-size render test takes minutes.

## Layout

Captured from the real binary in a 120×30 tmux pane (re-captured after the
narrow-header, wide-text and notification changes, and unchanged by them):

```
╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ in    ⌕ Search                                 1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ You ─────────────────╮╭────────────────────────────────────────────────────────────────╮╭ WorkedIn News ─────────────╮
│ Jordan Vale          ││ ✎ Start a post  (press p)                                      ││ Top stories                │
│ Software Engineer at │╰────────────────────────────────────────────────────────────────╯│ · Tabs-versus-spaces truce │
│ Quillfeather Systems │╭────────────────────────────────────────────────────────────────╮│   enters its third week    │
│                      ││ Maren Holt · 2h                                                ││   2h ago · 4,812 readers   │
│ Connections      312 ││ Staff Engineer at Quillfeather Systems                         ││ · Offices report record    │
│ Saved jobs         1 ││ I'm happy to share that our team cut the CI queue from 40      ││   demand for standing      │
╰──────────────────────╯│ minutes to 6.                                                  ││   desks                    │
                        │ No heroics: we cached the dependency build, split the sl …more ││   5h ago · 2,960 readers   │
                        │ 342 reactions · 3 comments                                     ││ · Study: stand-ups shorter │
                        │ Like  Comment  Repost  Send                                    ││   when chairs are removed  │
                        ╰────────────────────────────────────────────────────────────────╯│   9h ago · 1,745 readers   │
                        ╭────────────────────────────────────────────────────────────────╮│ · Terminal apps make a     │
                        │ Tobias Ferreira-Lund · 4h                                      ││   quiet comeback           │
                        │ Product Manager at Brightkettle                                ││   20h ago · 1,203 readers  │
                        │ Unpopular opinion: the best meeting is the one that turns into ││ · Four-day sprints win     │
                        │ a doc.                                                         ││   fans among small teams   │
                        │ Write the decision down, share it, and give people a day …more ││   1d ago · 986 readers     │
                        │ 1,204 reactions · 2 comments                                   │╰────────────────────────────╯
                        │ Like  Comment  Repost  Send                                    │
                        ╰────────────────────────────────────────────────────────────────╯
                        ╭────────────────────────────────────────────────────────────────╮
                        │ Ines Okonkwo · 6h                                              │
                        │ Technical Recruiter at Harborline Talent                       │
                        │ We're hiring! Harborline Logistics is looking for a Site       │
                        ╰────────────────────────────────────────────────────────────────╯
 j/k move  l like  enter open  p post  tab next  1-5 tabs  q quit
```

**No emoji, on purpose.** Every glyph is a single-width character:
`⌕ ✎ · … ● ▌ ┃ ▏ ◷ ★ ☆ ✔ ─`. Emoji such as 👍 or 💬 render two cells wide in
most terminals, and in tmux, so a line holding one is a column longer than
ratatui thinks it is. That pushes the right border of every bordered card out
of line. Don't add them. A unit test checks every seed string, every hint and
every rendered cell.

**Typed text is the exception.** You can type `日本語` or an emoji into the
composer, the reply box or the search box, so everything in `ui.rs` that
wraps, truncates or measures text uses display width (`width_of`, which is
ratatui's `Span::width`), never the `char` count. `head` and `tail` keep the longest
start or end of a string that fits in N columns. `hard_wrap` splits typed text
into rows of at most N columns, giving a character wider than the row a row of
its own. `wrap` and `truncate` are built on them. Seed text is single width,
so for it the two measures agree.

The screen is a header (3 rows), a body, and a footer (1 row).

- **Header:** the `in` logo in white on LinkedIn blue, a dimmed `⌕ Search`
  box (it shows the query, `⌕ bright▏`, while search is open), and the tabs
  `1 Home  2 My Network  3 Jobs  4 Messaging  5 Notifications`. The active tab
  is white, bold and underlined.
  - **Narrow terminals:** the logo, all five tab numbers and every non-zero
    badge always show. `header_fit` picks the first layout that fits:
    1. full labels plus a search box of at least 10 columns (from 90 columns
       with the seed badges),
    2. full labels without the search box, right-aligned (from 78),
    3. short labels: `1 Home  2 Network (3)  3 Jobs  4 Msgs (2)  5 Notifs (3)`
       (from 63),
    4. numbers only, except the active tab, which keeps its short label:
       `1 Home  2 (3)  3  4 (2)  5 (3)` (from 38 on Home, a little less or
       more on other tabs),
    5. bare numbers one space apart: `1 2 (3) 3 4 (2) 5 (3)` (from 29).

    Below 29 columns even that doesn't fit, so the tabs are clipped on the
    right and the logo and first tabs stay.
  - **Badges:** My Network shows the number of pending invitations, Messaging
    the number of unread conversations, Notifications the number of unread
    notifications, in red: `4 Messaging (2)`. A zero count shows no badge.
    `App::badge` computes them from the data on every frame, so they are never
    stale.
- **Footer:** key hints for the current tab and mode. A toast replaces them
  (white on blue) until the next key press. Every hint set fits on one line at
  80 columns. There are nine sets: Home, post view, composer, My Network,
  Jobs, Messaging list, reply box, Notifications and search.
- **Selection:** the selected list row has a blue `▌` bar and a highlighted
  background. The selected feed card and the focused box have a light blue
  border. Lists scroll only once the selection would fall off the bottom.

### Home

- **Left, profile card** (`PROFILE_WIDTH = 24`), titled "You": your name,
  wrapped headline, `Connections` and `Saved jobs`. Saved jobs is counted from
  the jobs on every frame, and Connections goes up when you accept an
  invitation.
- **Center:** a "Start a post (press p)" box above the feed. Each card shows:
  - `Author · 2h` (ages read `now`, `Nh`, `Nd` or `Nw`),
  - the author's headline, truncated with `…`,
  - the body, word-wrapped and cut to 3 rows, the last ending in `…more`,
  - `N reactions · M comments` (singular for 1, thousands with commas),
  - `Like  Comment  Repost  Send`, where Like becomes a blue `Liked`.
- **Right, "WorkedIn News"** (`SIDEBAR_WIDTH = 30`): "Top stories", each with
  its age and reader count.
- **Responsive:** the right sidebar shows only at 100 columns or more, and the
  profile card only at 80 or more. Below 80 only the feed is left.
- **Post view (Enter):** the post replaces the feed column. It shows the full
  body with newlines kept, the counts and action row, a rule, then the
  comments. `j`/`k` select a comment (marked with a blue `┃`), and the view
  scrolls just far enough to show the end of the selected one. A post with no
  comments says "No comments yet".
- **Composer (p):** a "Create a post" box centered over the body. It shows
  `Jordan Vale · Post to Anyone`, then the draft with a `▏` cursor, or the
  placeholder "What do you want to talk about?". The draft is split into rows
  by display width (`hard_wrap`) rather than word-wrapped, so every typed space
  shows. A long draft keeps its end, where the cursor is, in view.

### My Network

- A left "Manage my network" card (at 80 columns or more): Connections,
  Invitations and Sent (pending requests).
- **"Invitations (N)"**: name with `Ignore` / `Accept` on the right, then
  `headline · N mutual`. With none left it reads "No pending invitations" and
  the title drops the count.
- **"People you may know"**: name with `+ Connect` or `◷ Pending`, then the
  headline and mutual count.
- The selection is one index over invitations followed by suggestions, so `j`
  walks off the last invitation into the first suggestion. The box holding the
  selection gets the light blue border. The invitations box gets at most half
  the height, so both sections always show.

### Jobs

A "Jobs for you" list (40%) and "Job details" (60%).

- **List rows:** title in blue with `★` when saved, company, then
  `location · Remote/Hybrid/On-site`, plus a green `Applied` once applied.
- **Details:** title, company, `location · workplace`, `Posted Nd ago · N
  applicants`, then `Easy Apply` (or a green `✔ Applied`) and `☆ Save` (or
  `★ Saved`), then "About the job" and the description.

### Messaging

A conversation list (34 columns, at most half the width) and the open thread.

- **List rows:** a blue `●` and bold name for unread, the age of the last
  message, and a preview of it (`You: …` when it's yours).
- **Thread:** blank until you open one ("Select a conversation and press enter
  to open it"). Then: name and headline, a rule, the messages (`Name · age`,
  then the indented body, newest at the bottom), and a "Reply" box. The reply
  box shows "Press enter to reply" when the list has focus, and the draft with
  a `▏` cursor when it has focus. A long reply shows as much of its end as
  fits (`tail`, by display width), so the cursor never runs off the box.
- The thread on the right is the conversation you opened with Enter, not the
  one under the cursor. Moving the cursor doesn't open, and so doesn't mark
  read, anything.

### Notifications

One list: the text and its age. Opening the tab marks every notification read,
so the badge clears at once, as LinkedIn's bell does. The ones that were
unread when you opened the tab stay marked as new, with a blue `●` and bold
text, for the rest of that visit. `j`/`k`, or pressing `5` again, don't end
the visit. Leaving the tab, by any route, clears the markers, so the next
visit shows none until something new arrives (in this seeded app, nothing
does).

### Search

`/` opens a "Search" box centered over the body (at most 80×24, and 4
columns and 2 rows smaller than the body), over whatever tab and mode were
showing. Captured from the real binary in a 120×30 tmux pane after typing
`bright`:

```
╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ in    ⌕ bright▏                                1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ You ─────────────────╮╭────────────────────────────────────────────────────────────────╮╭ WorkedIn News ─────────────╮
│ Jordan Vale       ╭ Search ──────────────────────────────────────────────────────────────────────╮ies                │
│ Software Engineer │ ⌕ bright▏                                                          5 results │ersus-spaces truce │
│ Quillfeather Syste│ ──────────────────────────────────────────────────────────────────────────── │ its third week    │
│                   │  People                                                                      │ · 4,812 readers   │
│ Connections      3│ ▌Baptiste Moreau                                                             │s report record    │
│ Saved jobs        │ ▌Frontend Engineer at Brightkettle · Invitation                              │ for standing      │
╰───────────────────│  Mateo Achebe                                                                │                   │
                    │  Developer Advocate at Brightkettle · Connect                                │ · 2,960 readers   │
                    │                                                                              │ stand-ups shorter │
                    │  Jobs                                                                        │hairs are removed  │
                    │  Developer Advocate                                                          │ · 1,745 readers   │
                    │  Brightkettle · Anywhere in Europe · Remote                                  │al apps make a     │
                    │                                                                              │comeback           │
                    │  Posts                                                                       │o · 1,203 readers  │
                    │  Tobias Ferreira-Lund · Product Manager at Brightkettle                      │ay sprints win     │
                    │  Unpopular opinion: the best meeting is the one that turns into a doc. Writ… │mong small teams   │
                    │  Hana Sorensen · Data Scientist at Brightkettle                              │ · 986 readers     │
                    │  Our churn model hit 99% accuracy on the first try. Exciting, until we noti… │───────────────────╯
                    │                                                                              │
                    │                                                                              │
                    │                                                                              │
                    │                                                                              │
                    │                                                                              │
                    ╰──────────────────────────────────────────────────────────────────────────────╯
                        ╰────────────────────────────────────────────────────────────────╯
 type search  ↑/↓ move  enter open  esc close
```

- **Input row** (`search_input`): `⌕ `, the query and a `▏` cursor, and on
  the right the result count, `5 results` or `1 result`. There is no count
  for an empty query, and none next to `No results`. A query too wide for the
  box shows its end (`tail`, by display width). The count goes first: it only
  shows while the query keeps at least `QUERY_MIN_WIDTH` (10) columns, or all
  of a shorter one, so `⌕ bright▏  5 results` needs 20 columns and at 19 it
  is just `⌕ bright▏`. Under the input row, a rule.
- **Results:** the empty prompt `Type to search people, jobs and posts`, or
  `No results`, or the results grouped under `People`, `Jobs` and `Posts`
  headings, in that order. A group with no results has no heading. Each
  result is two lines:
  - person: the name, then `headline · Invitation` (or `· Connect` /
    `· Pending` for a suggestion). The label always shows; the headline is cut
    with `…` to make room,
  - job: the title, then `company · location · Remote/Hybrid/On-site`,
  - post: `author · headline`, then the body on one line, newlines shown as
    spaces, cut with `…`. When that line would hide the body's first match
    (`body_line`), it shows an excerpt instead: `…`, a few words before the
    match, the match, then as much as fits, cut with `…`. `skipped` shows
    `…a suite that had been skipped since before most of us joined. Small
    borin…` rather than the start of Maren Holt's post. The context gets up to
    a third of the row; when the rest of the body fits after the match and
    leaves more than that, the context takes all of it and the end shows
    uncut. `excerpt_start` starts the context on a word, so no word is cut at
    the start, except when the nearest word start doesn't fit (a long word,
    or CJK text, which has no spaces): then it keeps as much of the end as
    fits, by display width. A match already visible from the start keeps the
    plain line.
- **Highlight:** on each line, the first match of the query in a searched
  field (not in a separator or label) that fits before the `…` is bold.
  Nothing else in the results is bold, so bold always means "match".
  `match_ranges` finds matches in `text.to_lowercase()` and maps them back to
  whole characters of the original, because lowercasing can change length
  (`İ` becomes `i` plus a combining dot).
- **Selection:** the selected result gets the usual `▌` bar and background,
  and the list scrolls with `first_visible` (via `render_list`) to keep it in
  view. Headings are list items too, but never selected.
- **Header:** when the header has room for its search box, the box shows the
  end of the query and the cursor. `header_fit` doesn't look at search, so the
  header lays out the same with search open or closed.

## Data model (`src/data.rs`)

```rust
struct Profile      { name, headline, connections: u32 }
struct Post         { author, headline, hours_ago: u32, body: String,
                      reactions: u32, comments: Vec<Comment>, liked: bool }
struct Comment      { author, headline, hours_ago: u32, body }
struct News         { title, hours_ago: u32, readers: u32 }
struct Person       { name, headline, mutual: u32 }
struct Suggestion   { person: Person, pending: bool }
struct Job          { title, company, location, workplace: Workplace,
                      days_ago: u32, applicants: u32, description,
                      saved: bool, applied: bool }
enum   Workplace    { Remote, Hybrid, OnSite }
struct Conversation { name, headline, messages: Vec<Message>, unread: bool }
struct Message      { from_me: bool, body: String, hours_ago: u32 }
struct Notification { text, hours_ago: u32, unread: bool, is_new: bool }
```

- **Strings are `&'static str`** except `Post::body` and `Message::body`,
  which are `String` because published posts and replies hold typed text.
- **Ages** are stored as `hours_ago` (or `days_ago` for jobs), not
  timestamps, so rendering and the tests are deterministic. What you publish
  or send has age 0, shown as `now`.
- **`Notification::unread` drives the badge, and `is_new` drives the `●`.**
  `is_new` is set from `unread` when you enter the tab and cleared when you
  leave it. Both are `false` in the seed except `unread` on three items.
- **The comment count is `comments.len()`.** Unlike workit, there is no
  separate displayed total, so the card and the post view always agree.
- **Seed:** you are "Jordan Vale" with 312 connections. 9 posts, 5 news items,
  3 invitations, 4 suggestions, 6 jobs (1 saved), 4 conversations (2 unread)
  and 5 notifications (3 unread). Companies (Quillfeather Systems,
  Brightkettle, Pinecrest Studio, Harborline, Tinderbox Robotics, Larkspur
  Labs) and people are invented. Cities are real.

## App state (`src/app.rs`)

| Field | Meaning |
|---|---|
| `profile`, `posts`, `news`, `invitations`, `suggestions`, `jobs`, `conversations`, `notifications` | seed data, mutated in place |
| `tab: Tab` | `Home`, `Network`, `Jobs`, `Messaging`, `Notifications` |
| `mode: Mode` | `Browse`, `PostDetail { comment }`, `Composer`, `Reply` |
| `post_sel`, `network_sel`, `job_sel`, `chat_sel`, `notif_sel` | one selection per list, kept while you visit other tabs |
| `open_chat: Option<usize>` | the thread shown in Messaging. `None` until you press Enter |
| `draft`, `reply` | composer and reply box text |
| `search: Option<Search>` | `Some { query, selected }` while search is open. `selected` indexes `App::search_hits` |
| `status: Option<&'static str>` | toast text. Every toast is a fixed string. Cleared on the next key press |
| `should_quit` | checked by the event loop in `main.rs` |

`App` does no I/O. `App::new()` seeds it from `data.rs`, and
`App::on_key(KeyEvent)` is the only way it changes.

- **Derived counts:** badges (`App::badge`) and saved jobs
  (`App::saved_jobs`) are computed when asked, never stored.
- **Search results** are never stored either. `App::search_hits(query)`
  recomputes them from the current state each time, as `SearchHit` values
  (`Invitation(i)`, `Suggestion(i)`, `Job(i)`, `Post(i)`) indexing the live
  lists. The query is trimmed and lowercased (`search_needle`), and a field
  matches when its `to_lowercase()` contains that. Fields: invitation and
  suggestion name and headline; job title, company and location; post author,
  headline and body. Comments, conversations, notifications and news aren't
  searched. A query that is empty once trimmed matches nothing.
- **Search sits on top of `mode`.** Opening it leaves `tab`, `mode` and every
  selection alone, so Esc (`search = None`) puts back exactly what was there,
  including the post view and its selected comment.
- **Switching tabs** (with `1`-`5`, `Tab` or `BackTab`) closes the post view,
  so coming back to Home shows the feed with the same post selected.
  Entering Notifications from another tab copies each `unread` into `is_new`
  and clears `unread`. Leaving it clears `is_new`. Pressing `5` while already
  there changes nothing.
- **Removing an invitation** keeps `network_sel` and clamps it to the list. The
  next row slides under the cursor, or the cursor moves up when the last row
  goes. An empty list leaves it at 0, where `a`, `x` and `c` do nothing.
- **Publishing** inserts the post at index 0 and sets `post_sel` to 0. The
  published body is the draft with surrounding whitespace trimmed.
- **Text boxes:** a key is typed when it is `KeyCode::Char` without Ctrl or
  Alt. Shift is fine.

## Key bindings

**Global:** Ctrl-C quits from any mode, including the text boxes.

Everywhere except the composer, the reply box and search:

| Key | Action |
|---|---|
| `1`-`5` | switch tab |
| `Tab` / `Shift-Tab` | next / previous tab, wrapping both ways |
| `j`/`↓`, `k`/`↑` | move the selection in the current list (comments in the post view) |
| `/` | open search (also from the post view) |
| `Esc` | close the post view. Otherwise nothing, so it never quits |
| `q` | quit |

Per tab:

| Tab | Key | Action |
|---|---|---|
| Home | `l` | like / unlike the selected post, reactions ±1. Also in the post view |
| Home | `Enter` | open the post view |
| Home | `p` | open the composer (from the feed, not the post view) |
| My Network | `a` | accept the selected invitation: removed, connections +1, "Invitation accepted" |
| My Network | `x` | ignore it: removed, connections unchanged, "Invitation ignored" |
| My Network | `c` | on a suggestion, Connect ↔ Pending: "Invitation sent" / "Invitation withdrawn" |
| Jobs | `s` | save / unsave: "Job saved" / "Job unsaved" |
| Jobs | `e` | Easy Apply: "Application sent", then "Already applied" with no change |
| Messaging | `Enter` | open the selected conversation, mark it read, focus the reply box |

`a`/`x` on a suggestion and `c` on an invitation do nothing, not even a
toast. Opening Notifications by any route marks them all read.

**Composer:** every printable key, including `q`, `j`, `k`, `l`, `1`-`5`,
`a`, `x`, `c`, `s` and `e`, is typed. `Backspace` deletes. `Enter` publishes
and toasts "Post published", or toasts "Write something first" and stays open
when the draft is empty or only whitespace. `Esc` discards the draft. `Tab`
does nothing.

**Search:** a text box like the composer. Every printable key without Ctrl
or Alt is typed, including `q`, `j`, `k`, `l`, `p`, `a`, `x`, `c`, `s`, `e`,
`/` and `1`-`5`, and does nothing else. `Backspace` deletes, and does nothing
on an empty query (search stays open). `↓`/`↑` move through the results,
stopping at both ends. Any change to the query selects the first result
again. `Enter` closes search and goes to the selected result, switching tabs
the way `1`-`5` do (so leaving Notifications ends the visit):

| Result | Lands on |
|---|---|
| invitation `i` | My Network, `network_sel = i` |
| suggestion `i` | My Network, `network_sel = invitations + i` |
| job `i` | Jobs, `job_sel = i` |
| post `i` | Home feed (not the post view), `post_sel = i` |

`Enter` with no results, or an empty query, toasts "No results" and keeps
search open with the query. `Esc` closes it and discards the query. `Tab`,
`BackTab` and every Ctrl- or Alt-modified key do nothing (Ctrl-C still
quits).

**Reply box:** printable keys are typed the same way. `Enter` appends the
reply to the thread as yours, clears the box and keeps focus. An empty or
whitespace-only reply does nothing. `Esc` discards the text and returns focus
to the list.

## Files

```
workedin/
  PLAN.md
  Cargo.toml    ratatui 0.30 only, edition 2024
  Cargo.lock
  src/
    main.rs     ratatui::init()/restore() plus cursor::Show, the panic hook, the event loop
    app.rs      state, key handling, badges, search. No rendering, no I/O
    data.rs     seed profile, posts, news, people, jobs, conversations, notifications
    ui.rs       draw(frame, &App): all rendering, pure
```

**Terminal restore:** `ratatui::init()` enables raw mode, enters the alternate
screen and installs a panic hook that calls `ratatui::restore()`. `restore()`
doesn't show the cursor, so `main.rs` wraps that hook to also send
`cursor::Show`, and does the same after a normal quit. Both paths were checked
in tmux: after `q`, and after a temporarily injected panic, `stty` matched
its state from before launch, the alternate screen was off and the cursor was
visible.

## Testing

62 unit tests, run with `cargo test` (about 6 s):

- **`app.rs`** (29), on the seed data:
  - Like adds 1 to reactions and a second press restores the count.
  - The post view opens on Enter, `j`/`k` walk its comments, and Esc returns
    to the same post. Switching tabs closes it.
  - Composer: `q`, `1`, `j`, `l`, `a`, `x`, `c`, `s`, `e` and space are typed
    and do nothing else, and `Tab` doesn't switch tabs. Backspace deletes. Esc
    discards. Empty and whitespace-only publishes are rejected with the toast
    and the composer stays open. Publishing puts the post first, authored by
    you at age 0, and selects it.
  - Accept and ignore: connection count and My Network badge. Removing the
    last invitation, with and without suggestions after it, keeps the
    selection in bounds. `c` on an invitation and `a`/`x` on a suggestion do
    nothing. `c` toggles Connect/Pending with both toasts.
  - Easy Apply twice: the second press only toasts "Already applied". Saving a
    job changes the saved count by one each way.
  - Opening an unread conversation decrements the Messaging badge. Sending
    appends a message from you. An empty send does nothing. Digits and `q`
    typed in the reply box don't switch tabs or quit.
  - Opening Notifications, with `5` or `BackTab`, clears its badge.
  - The notifications that were unread are `is_new` for the whole visit,
    including after `j` and a second `5`. Leaving with `1` or `Tab` clears
    them, and the next visit has none, with the badge still at 0.
  - `Tab` and `BackTab` wrap in both directions.
  - `q` quits, Esc doesn't, and toasts clear on the next key.
  - Search, opening and closing: `/` opens it from Browse on every tab and
    from the post view. Esc restores the tab, the mode (with the selected
    comment), all five selections, the open thread and the Notifications
    markers. `/` is typed in the composer and the reply box.
  - Search typing: `q1jlpaxcse5/` on My Network is typed, with no quit, tab
    switch, move, accept, ignore or connect. Shift is fine. `Tab`, `BackTab`
    and Ctrl/Alt keys change nothing. Backspace on an empty query keeps search
    open. Ctrl-C quits.
  - `BRIGHT`, `brightkettle` and `  bRiGhT ` give exactly Baptiste Moreau,
    Mateo Achebe, Developer Advocate at Brightkettle, then Tobias
    Ferreira-Lund's and Hana Sorensen's posts. Headlines, bodies and
    locations match. Comments, news and blank queries don't.
  - Live results: an accepted invitation drops out, a published
    `Rust meetup at Brightkettle` is the first post result, and a connected
    suggestion is pending.
  - `Enter` on each kind of result lands on the right tab and selection; the
    post jump lands in the feed even from the post view. No results toasts
    "No results" and stays open. The selection clamps at both ends and goes
    back to the first result when the query changes. Jumping from
    Notifications clears its markers, Esc doesn't.
- **`ui.rs`** (33), rendering into a `TestBackend`:
  - The helpers `age`, `thousands`, `count`, `wrap`, `hard_wrap`,
    `truncate` and `first_visible`, plus `width_of`, `head` and `tail`. The
    text helpers are also checked on double-width text.
  - At 120×30, Home shows all three columns, in order. At 90×30 the sidebar
    is gone, and at 70×30 the profile card is too. The exact thresholds: the
    sidebar shows at 100 but not 99, and the card at 80 but not 79.
  - Every size from 1×1 to 140×40 renders without panicking, for 15 states:
    Home, post view, composer (empty and with text), My Network (invitation
    and suggestion selected), Jobs (with and without a toast), Messaging,
    reply box, Notifications, and search (with matches, empty, no match, and
    over the post view). One thread per state.
  - Card counts and `Like` → `Liked`. A published post renders first with
    `now`. The post view shows the full body and comments.
  - Badges in the header follow every count and disappear at 0.
  - Narrow header: at 80×24, 70×30 and 60×20 the header row starts with the
    logo and ends with the expected compact tabs, with all five numbers and
    the badges `(3)`, `(2)`, `(3)`, and no search box. 120×30 keeps the
    search box. With only numbers, the active tab keeps its name.
  - For every state and every width from 29 to 140, the header shows the logo,
    the numbers `12345` and exactly the non-zero badges.
  - Wide typed text (`a日本語…`, odd-width so a wide character lands on a row
    boundary) at 40×16: the composer's side borders are intact and the whole
    draft is visible inside the box, and after publishing every body row ends
    in a card border and the card shows the whole text. At 60×20 the reply box
    shows the end of the reply, filling the box to within one column, with
    every border in place, and the sent message wraps inside the thread
    in full. Making `width_of` count `char`s instead fails these tests.
  - Notifications: on a visit the badge is gone and exactly the three
    previously unread items show `●`, still after `j`. After leaving and
    coming back no `●` is left.
  - The profile card's saved jobs and connections follow the Jobs and My
    Network tabs.
  - My Network, Jobs and Messaging contents, including `◷ Pending`,
    `✔ Applied`, unread markers and the reply box.
  - Each of the nine hint sets is at most 80 columns, and is what the footer
    shows at 80×24.
  - No seed string, tab label or hint contains a character that isn't single
    width, and no rendered cell on any screen holds one. The same goes for
    every search label.
  - Search overlay at 120×30: the input row, the headings in order, each
    result's two lines (including the `· Invitation` / `· Connect` labels and
    a body cut with `…`), the header box showing the query, no heading for an
    empty group, the empty prompt (also for a blank query) and `No results`.
    A long result list scrolls to the selection, and a connected suggestion
    shows `· Pending`.
  - Result count: `5 results` for `bright`, `1 result` for `lisbon`, nothing
    for an empty query or next to `No results`, and a 68-column query keeps
    both its end and `1 result` at 120 columns. `search_input` at every width
    from 40 down to 3: once the count is gone it doesn't come back, and
    whenever it shows, all of `bright` shows too (20 columns with the count,
    19 without), and a long query keeps at least 10 columns of its end next to
    the count and fills the row without it.
  - Excerpts: `excerpt_start` starts on a word, never on a space, and cuts
    only a word, or CJK text, wider than the budget. `body_line` matches the
    plain line when the match is visible (or absent), and otherwise gives
    `…`, whole words, the bold match and `…`, also for `İstanbul`, and never
    wider than the row at any width up to 40. Rendered: `skipped` gives an
    excerpt starting on a word of Maren Holt's body with only `skipped` bold.
    A match at the end of a long published body shows that end uncut, at
    most one word short of the row. In a CJK body, `東京駅` is the only bold
    run and the row fits the box; when the rest fits it shows uncut (at 120
    and 60 columns), and a long rest is cut with `…`. `CI queue`, visible
    from the start, keeps `I'm happy to share…`.
  - Highlighting: `match_ranges` on `İstanbul`, overlapping matches, a final
    sigma and CJK, and `hit_line` only bolding a match that fits before the
    `…` and is in a searched part. With `İstanbul office opening` published,
    `stan` finds the post and the only bold cells read exactly `stan` (at 120,
    60 and 30 columns, and none at 12, where the match is cut off). `日本語`
    bolds exactly `日本語`, and `BRIGHT` gives five bold runs, all `Bright`.
  - For an empty query, `bright`, a 3,000-character query mixing `日本語` and
    ASCII, and a query matching nothing, every size from 1×1 to 140×40
    renders, and every row of the overlay starts and ends with its border and
    is exactly the box's width, measured by display width. The long query
    shows its end in the input row and the header box, and the header keeps
    the logo, all five numbers and the badges at 140, 120, 80, 70 and 60
    columns.

These were checked by breaking the code on purpose and restoring it:
case-sensitive matching fails 8 tests (including the `BRIGHT` one), taking
highlight offsets from the lowercased string fails the `match_ranges` and
`İstanbul` highlight tests, making Esc fall back to Browse fails the Esc
restore test, not resetting the selection when typing fails the selection
test, and showing the start of a long query instead of its end fails the
every-size border test. For the excerpt and the count: starting the context
wherever the width allows instead of on a word fails 4 tests (including
`excerpt_starts_on_a_word_unless_the_word_is_too_wide` and the `skipped`
one), always showing the plain body line fails 4 (including the `skipped`,
end-of-body and CJK ones), measuring the rest of the body by `char` count
fails the CJK excerpt test, and keeping the count however narrow the row
fails `the_count_goes_before_the_end_of_the_query`.

## Ideas

Comment and repost actions, searching comments and messages, scrolling long
job descriptions, profile pages, mouse support.
