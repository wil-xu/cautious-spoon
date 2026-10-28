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
  comments, commenting on it and deleting your own comments, sending a post
  to a conversation, writing a post, accepting or ignoring invitations,
  sending connection requests, saving jobs and applying, reading and replying
  to messages, and clearing notifications. `/` searches people, jobs, posts,
  comments and messages and jumps to the result.
- One self-contained binary. No network, no files written at runtime and no
  real data: all content is hard-coded, fictional and safe for work.

## Non-goals

- Real LinkedIn data or accounts (would need auth and APIs, and raises privacy
  concerns).
- Persisting state between runs. Likes, posts, comments, connections, saved
  jobs and messages reset on restart.
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
narrow-header, wide-text and notification changes, and unchanged by them;
re-captured again after the Send picker, which only added `s send` to the
footer):

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
 j/k move  l like  enter open  p post  s send  tab next  1-5 tabs  q quit
```

**No emoji, on purpose.** Every glyph is a single-width character:
`⌕ ✎ · … ● ▌ ┃ ▏ ◷ ★ ☆ ✔ ─`. Emoji such as 👍 or 💬 render two cells wide in
most terminals, and in tmux, so a line holding one is a column longer than
ratatui thinks it is. That pushes the right border of every bordered card out
of line. Don't add them. A unit test checks every seed string, every hint and
every rendered cell.

**Typed text is the exception.** You can type `日本語` or an emoji into the
composer, the reply box, the comment box, the search box or the Send
picker's filter, so everything in `ui.rs` that
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
  80 columns. There are eleven sets: Home, post view, comment box, composer,
  My Network, Jobs, Messaging list, reply box, Notifications, search and the
  Send picker. Jobs reads
  `j/k move  s save  e easy apply  tab next  1-5 tabs  q quit  J/K scroll`
  (72 columns with the padding). A narrower terminal cuts every set on the
  right. `J/K scroll` comes last so that a Jobs footer narrower than 72
  columns shows what it did before the details could scroll: everything up
  to `q quit` from 60 columns, cut in the same place below that.
- **Selection:** the selected list row has a blue `▌` bar and a highlighted
  background. The selected feed card and the focused box have a light blue
  border. Lists scroll only once the selection would fall off the bottom.
- **Overlays** (the composer, search and the Send picker) clear their box
  with `clear_overlay`, which also blanks a double-width character just left
  of the box. The terminal draws both halves of such a character, and
  ratatui's buffer diff skips the cell after it, so the box's left border
  cell was never sent: with a published post of wide text under the left
  edge, the border went missing in about 300 rows across sizes, first at
  134×13.

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
  - `Like  Comment  Repost  Send`, where Like becomes a blue `Liked`. Like
    (`l`), Comment (`c` in the post view) and Send (`s`) work. Repost is only
    a label.
- **Right, "WorkedIn News"** (`SIDEBAR_WIDTH = 30`): "Top stories", each with
  its age and reader count.
- **Responsive:** the right sidebar shows only at 100 columns or more, and the
  profile card only at 80 or more. Below 80 only the feed is left.
- **Post view (Enter):** the post replaces the feed column. It shows the full
  body with newlines kept, the counts and action row, a rule, then the
  comments, oldest first. `j`/`k` select a comment (marked with a blue `┃`).
  A post with no comments says "No comments yet" instead of the `Comments`
  heading.
  - **"Add a comment" box:** always at the bottom of the Post card, inside its
    border (`post_detail_areas` splits the card's inside into the comments and
    a 3-row box). It is drawn like the Messaging reply box
    (`render_comment_box`): a dim `Press c to comment` until `c` focuses it,
    then the draft followed by a `▏` cursor, or the dim placeholder
    `Add a comment…` followed by the cursor when the draft is empty. A draft
    too wide for the box shows its end (`tail`, by display width), so the
    cursor and what you just typed stay in view. Its border is light blue
    while it has focus. In a box too narrow for the whole placeholder, the
    placeholder is cut (`head`) so the cursor still shows.
  - **Scrolling:** the comments scroll in the rows above the box, never under
    it: the view scrolls just far enough to show the end of the selected
    comment above the box. A comment you post is selected, so it ends right
    above the box.
  - **Short cards:** the box shows only when it fits whole: the card's inside
    needs 3 rows and 5 columns. The post and comments get whatever rows are
    left, possibly none, so a focused box stays visible as long as possible.
    On a smaller card only the post and comments show. At 120×30, 80×24 and
    60×20 the box always shows in full.
  - **Deleting your comments:** `x` removes the selected comment if you wrote
    it and toasts "Comment deleted". The count, on the card too, follows, and
    deleting a post's only comment brings back "No comments yet". `x` on
    someone else's comment does nothing, and every seed comment is someone
    else's.

  Captured from the real binary in a 120×30 tmux pane, after posting
  `Agreed: q1 caching / split shards is the way` on Maren Holt's post and
  pressing `c` again to start another comment:

  ```
  ╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
  │ in    ⌕ Search                                 1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
  ╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  ╭ You ─────────────────╮╭ Post ──────────────────────────────────────────────────────────╮╭ WorkedIn News ─────────────╮
  │ Jordan Vale          ││ 342 reactions · 4 comments                                     ││ Top stories                │
  │ Software Engineer at ││ Like  Comment  Repost  Send                                    ││ · Tabs-versus-spaces truce │
  │ Quillfeather Systems ││ ────────────────────────────────────────────────────────────── ││   enters its third week    │
  │                      ││ Comments                                                       ││   2h ago · 4,812 readers   │
  │ Connections      312 ││                                                                ││ · Offices report record    │
  │ Saved jobs         1 ││   Felix Abernathy · 1h                                         ││   demand for standing      │
  ╰──────────────────────╯│   Software Engineering Intern at Quillfeather Systems          ││   desks                    │
                          │   Can confirm. I pushed twice before lunch today and both runs ││   5h ago · 2,960 readers   │
                          │   finished.                                                    ││ · Study: stand-ups shorter │
                          │                                                                ││   when chairs are removed  │
                          │   Ravi Tenorio · 1h                                            ││   9h ago · 1,745 readers   │
                          │   Engineering Manager at Quillfeather Systems                  ││ · Terminal apps make a     │
                          │   The deleted suite is my favorite part. Nobody missed it.     ││   quiet comeback           │
                          │                                                                ││   20h ago · 1,203 readers  │
                          │   Hana Sorensen · now                                          ││ · Four-day sprints win     │
                          │   Data Scientist at Brightkettle                               ││   fans among small teams   │
                          │   Would love a write-up of the caching setup.                  ││   1d ago · 986 readers     │
                          │                                                                │╰────────────────────────────╯
                          │ ┃ Jordan Vale · now                                            │
                          │ ┃ Software Engineer at Quillfeather Systems                    │
                          │ ┃ Agreed: q1 caching / split shards is the way                 │
                          │ ╭ Add a comment ─────────────────────────────────────────────╮ │
                          │ │ Happy to pair on the write-up▏                             │ │
                          │ ╰────────────────────────────────────────────────────────────╯ │
                          ╰────────────────────────────────────────────────────────────────╯
   type write  backspace delete  enter post  esc cancel
  ```
- **Composer (p):** a "Create a post" box centered over the body. It shows
  `Jordan Vale · Post to Anyone`, then the draft with a `▏` cursor, or the
  placeholder "What do you want to talk about?". The draft is split into rows
  by display width (`hard_wrap`) rather than word-wrapped, so every typed space
  shows. A long draft keeps its end, where the cursor is, in view.
- **Send picker (s):** `s` in the feed or the post view opens a "Send" box
  over the body for the selected post, sized and placed like the search
  overlay (`search_area`). From the top (`render_send`):
  - **Filter row** (`send_input`): `To `, the filter and a `▏` cursor, or the
    dim placeholder `Type a name` and the cursor when the filter is empty. A
    filter too wide for the row shows its end (`tail`, by display width), so
    the cursor stays visible. In a row too narrow for the placeholder, the
    placeholder is cut (`head`).
  - **Post row:** `author · ` and the first line of the body (`first_line`),
    cut with `…` to fit (`hit_line` with no query, so nothing is bold).
  - A rule.
  - **Recipients:** the conversations whose name matches the filter, in
    Messaging list order, two lines each: the name, then the headline cut with
    `…`. Names are regular weight, and the first match of the filter in each
    name is bold, using search's `hit_line` and `match_ranges`, so bold means
    "match" here too and an empty filter bolds nothing. The selected one gets the usual `▌` bar and background, and the list
    scrolls with `render_list` to keep it in view. An empty filter lists every
    conversation, and a filter matching nothing shows `No matches`.

  Like the other overlays, it clears its box with `clear_overlay` (see
  "Overlays" above).

  Captured from the real binary in a 120×30 tmux pane, opened from the feed
  with Maren Holt's post selected, after typing `o` (re-captured after the
  bold change, which plain text doesn't show; the text is unchanged):

  ```
  ╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
  │ in    ⌕ Search                                 1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
  ╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
  ╭ You ─────────────────╮╭────────────────────────────────────────────────────────────────╮╭ WorkedIn News ─────────────╮
  │ Jordan Vale       ╭ Send ────────────────────────────────────────────────────────────────────────╮ies                │
  │ Software Engineer │ To o▏                                                                        │ersus-spaces truce │
  │ Quillfeather Syste│ Maren Holt · I'm happy to share that our team cut the CI queue from 40 minu… │ its third week    │
  │                   │ ──────────────────────────────────────────────────────────────────────────── │ · 4,812 readers   │
  │ Connections      3│ ▌Maren Holt                                                                  │s report record    │
  │ Saved jobs        │ ▌Staff Engineer at Quillfeather Systems                                      │ for standing      │
  ╰───────────────────│  Ravi Tenorio                                                                │                   │
                      │  Engineering Manager at Quillfeather Systems                                 │ · 2,960 readers   │
                      │  Ines Okonkwo                                                                │ stand-ups shorter │
                      │  Technical Recruiter at Harborline Talent                                    │hairs are removed  │
                      │                                                                              │ · 1,745 readers   │
                      │                                                                              │al apps make a     │
                      │                                                                              │comeback           │
                      │                                                                              │o · 1,203 readers  │
                      │                                                                              │ay sprints win     │
                      │                                                                              │mong small teams   │
                      │                                                                              │ · 986 readers     │
                      │                                                                              │───────────────────╯
                      │                                                                              │
                      │                                                                              │
                      │                                                                              │
                      │                                                                              │
                      │                                                                              │
                      ╰──────────────────────────────────────────────────────────────────────────────╯
                          ╰────────────────────────────────────────────────────────────────╯
   type filter  ↑/↓ move  enter send  esc cancel
  ```

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
- **Scrolling the details:** on a short terminal the card doesn't hold the
  whole description (at 80×20, 4 of the 6 seed jobs; at 60×20, all 6). `J`
  and `K` scroll the whole card, header lines included, one line at a time,
  without moving the list. It stops at the top, and at the bottom once the
  last line of the description is on the card's last row, so it never shows
  blank rows below the end while lines above are hidden. When everything
  fits it shows from the top and `J`/`K` change nothing visible.
  - **Indicators:** while lines are hidden below, the bottom border ends in
    a light blue ` ↓ more ` just left of the `╯`. While lines are hidden
    above, the top border ends in ` ↑ more ` just left of the `╮`, with
    ` Job details ` still on the left. Each goes in only when it fits whole
    between the corners, and ` ↑ more ` only with at least one `─` between
    it and the whole title (from a 24-column card, a 40-column terminal;
    ` ↓ more ` needs a 10-column card, a 16-column terminal), so a narrow
    card drops it rather than cutting it or the title. A card
    one row high has its top and bottom border on the same row and gets
    neither.
  - **Reset:** the card goes back to the top whenever `j`/`k`/`↓`/`↑` move
    to another job, and on every search jump to a job, even to the one
    already selected. `j` on the last job and `k` on the first don't change
    `job_sel`, so they keep it. Switching tabs and coming back, `/` then
    Esc, and `s` and `e` keep it, like every other selection.

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
- **Shared posts:** a post sent from Home is a message from you like any
  reply: the list preview reads `You: Shared a post by …` with age `now`, and
  if that conversation is the open thread it shows at the bottom, wrapped in
  full. Sending doesn't mark the conversation read or unread, reorder the
  list, move the cursor, open a thread or touch the reply draft.

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
`typo`:

```
╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ in    ⌕ typo▏                                  1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ You ─────────────────╮╭────────────────────────────────────────────────────────────────╮╭ WorkedIn News ─────────────╮
│ Jordan Vale       ╭ Search ──────────────────────────────────────────────────────────────────────╮ies                │
│ Software Engineer │ ⌕ typo▏                                                            3 results │ersus-spaces truce │
│ Quillfeather Syste│ ──────────────────────────────────────────────────────────────────────────── │ its third week    │
│                   │  Posts                                                                       │ · 4,812 readers   │
│ Connections      3│ ▌Felix Abernathy · Software Engineering Intern at Quillfeather Systems       │s report record    │
│ Saved jobs        │ ▌Today I merged my first pull request! It fixes a typo in an error message … │ for standing      │
╰───────────────────│                                                                              │                   │
                    │  Comments                                                                    │ · 2,960 readers   │
                    │  Maren Holt · on Felix Abernathy's post                                      │ stand-ups shorter │
                    │  Welcome aboard. The typo had it coming.                                     │hairs are removed  │
                    │                                                                              │ · 1,745 readers   │
                    │  Messages                                                                    │al apps make a     │
                    │  Felix Abernathy                                                             │comeback           │
                    │  On to the next typo.                                                        │o · 1,203 readers  │
                    │                                                                              │ay sprints win     │
                    │                                                                              │mong small teams   │
                    │                                                                              │ · 986 readers     │
                    │                                                                              │───────────────────╯
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
- **Results:** the empty prompt `Type to search people, jobs, posts,
  comments and messages`, or `No results`, or the results grouped under
  `People`, `Jobs`, `Posts`, `Comments` and `Messages` headings, in that
  order. A group with no results has no heading. Each result is two lines:
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
    plain line,
  - comment: `commenter · on <post author>'s post`, then the comment body on
    one line by the same `body_line` rule as a post body (plain, or an
    excerpt around a hidden first match),
  - conversation: the name, then the message the hit carries
    (`SearchHit::Conversation { chat, message }`, chosen by
    `App::search_hits`; `hit_lines` only draws it). Your own message gets a
    dim `You: ` prefix, as on the Messaging list, and the body follows it by
    the `body_line` rule in the columns left (`prefixed_body_line`); when the
    prefix alone fills the row, prefix and body are cut together with `…`.
    With `thanks`, Ines Okonkwo shows
    `You: Thanks Ines, I'll keep it in mind.` (her own older message matches
    too) and Felix Abernathy shows his only match,
    `Thanks for reviewing my first PR!`, not his newest message. The row
    doesn't depend on `unread`: a read and an unread conversation look the
    same.
- **Highlight:** on each line, the first match of the query in a searched
  field (not in a separator or label) that fits before the `…` is bold. For
  the new kinds the searched fields drawn are a comment body, a conversation
  name and a message body. A comment's first line (commenter, `· on`, post
  author, `'s post`) and the `You: ` prefix are never bold, and a
  conversation found only by its name has no bold on its second line.
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
struct Comment      { author, headline, hours_ago: u32, body: String }
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

- **Strings are `&'static str`** except `Post::body`, `Message::body` and
  `Comment::body`, which are `String` because published posts, replies and
  comments hold typed text. A comment's author and headline are still
  `&'static str`: yours come from the profile.
- **Ages** are stored as `hours_ago` (or `days_ago` for jobs), not
  timestamps, so rendering and the tests are deterministic. What you publish,
  send or comment has age 0, shown as `now`.
- **`Notification::unread` drives the badge, and `is_new` drives the `●`.**
  `is_new` is set from `unread` when you enter the tab and cleared when you
  leave it. Both are `false` in the seed except `unread` on three items.
- **The comment count is `comments.len()`.** Unlike workit, there is no
  separate displayed total, so the card and the post view always agree,
  including right after you comment.
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
| `mode: Mode` | `Browse`, `PostDetail { comment }`, `CommentBox { comment }`, `Composer`, `Reply` |
| `post_sel`, `network_sel`, `job_sel`, `chat_sel`, `notif_sel` | one selection per list, kept while you visit other tabs |
| `job_scroll` | lines of the Jobs details card scrolled off the top, never more than `job_scroll_max`. Back to 0 when `job_sel` changes, kept like a selection otherwise |
| `job_scroll_max` | how far the selected job's details can scroll on the current screen, set by `fit_job_details` |
| `open_chat: Option<usize>` | the thread shown in Messaging. `None` until you press Enter |
| `draft`, `reply`, `comment_draft` | composer, reply box and comment box text |
| `search: Option<Search>` | `Some { query, selected }` while search is open. `selected` indexes `App::search_hits` |
| `send: Option<SendPicker>` | `Some { query, selected }` while the Send picker is open. `query` is the filter, `selected` indexes `App::send_recipients` |
| `status: Option<&'static str>` | toast text. Every toast is a fixed string. Cleared on the next key press |
| `should_quit` | checked by the event loop in `main.rs` |

`App` does no I/O. `App::new()` seeds it from `data.rs`, and
`App::on_key(KeyEvent)` is the only way it changes, apart from
`App::fit_job_details(max)` (below).

- **The job details scroll is clamped where it is stored.** If `App` kept an
  unclamped offset and `draw` clamped it, `J` past the end would bank
  presses that `K` then had to pay back before the text moved. But only the
  event loop knows the screen size, and only the layout in `ui.rs` knows
  how many rows the details take. So `ui::job_scroll_max(&App, Size)`, a
  pure function built on the same `screen_areas`, `jobs_areas`, `card` and
  `job_detail_lines` that `render_jobs` uses, measures the lines that don't
  fit, and `main.rs` hands that to `App::fit_job_details(max)` before every
  frame (so before every key, and after every resize). It stores `max` in
  `job_scroll_max` and pulls `job_scroll` down to it, so a terminal that
  grew is accounted for before the next key. `J` adds one up to
  `job_scroll_max`, `K` takes one off down to 0, so every key that can move
  the text does. `draw` still takes `&App`: it clamps `job_scroll` again
  against what fits in the frame it is drawing, which keeps a frame drawn
  before the app has heard of a resize free of blank rows too. In the real
  loop the two agree.

- **Derived counts:** badges (`App::badge`) and saved jobs
  (`App::saved_jobs`) are computed when asked, never stored.
- **Search results** are never stored either. `App::search_hits(query)`
  recomputes them from the current state each time, as `SearchHit` values
  (`Invitation(i)`, `Suggestion(i)`, `Job(i)`, `Post(i)`,
  `Comment { post, comment }`, `Conversation { chat, message }`) indexing
  the live lists.
  The query is trimmed and lowercased (`search_needle`), and a field matches
  when its `to_lowercase()` contains that. Fields: invitation and suggestion
  name and headline; job title, company and location; post author, headline
  and body; comment body (not its author or headline); conversation name and
  every message body (not its headline). Notifications and news aren't
  searched. The order is people (invitations, then suggestions), jobs, posts
  in feed order, comments (by post in feed order, then in post view order),
  then conversations in Messaging list order. A post and one of its comments
  are separate results: neither makes the other match. A conversation is one
  result however many of its messages match, and its `message` is the index
  of the message its row shows: the newest whose body matches, or the newest
  when only the name matches (`None` only for a conversation with no
  messages, which the seed and the app never make). "Newest" is last in the
  thread, where replies and shared posts are appended. The rule lives here,
  next to the matching, so a row can't show a message that isn't the match.
  A query that is empty once trimmed matches nothing.
- **One filter box for search and the picker.** `Search` and `SendPicker`
  are both `FilterBox { query, selected }`. `FilterBox::on_key(key, len)`
  does the editing and moving for both: Ctrl- and Alt-modified keys do
  nothing, a typed key is appended, `Backspace` deletes, and either change
  that alters the query selects the first item again; `↑`/`↓` move,
  stopping at 0 and `len - 1`; `Tab`, `BackTab` and every other key do
  nothing. It hands `Enter` and `Esc` back as a `BoxKey`, and
  `on_search_key` and `on_send_key` decide what they do (jump or send,
  close).
- **Search sits on top of `mode`.** Opening it leaves `tab`, `mode` and every
  selection alone, so Esc (`search = None`) puts back exactly what was there,
  including the post view and its selected comment.
- **The Send picker sits on top of `mode`** too. `s` opens it from
  `on_home_key`, so only on Home, in the feed or the post view, and only
  when there is a selected post. It leaves `tab`, `mode` and every selection
  alone, so Esc and a successful Enter (`send = None`) put back the feed with
  the same post or the post view with the same comment. Search and the picker
  are never open together: `/` is typed into the picker and `s` into search.
- **Recipients** are never stored. `App::send_recipients(filter)` returns the
  indexes of the conversations whose `name.to_lowercase()` contains
  `search_needle(filter)`, in list order. Headlines and messages aren't
  matched. A filter that is empty once trimmed lists every conversation.
- **Sending** (`send_post`) closes the picker and pushes a `Message` from you
  at age 0 onto the end of the recipient's messages, where the recipient is
  the selected row of the filtered list. The body is `Shared a post by
  <author>: <first line of the body>` (`first_line`, the text before the
  first newline; a published post has none, so it's the whole post). Then it
  toasts "Post sent". It changes nothing else: not `unread` (so the badge
  stays), the order of the conversations, `chat_sel`, `open_chat`, `reply`,
  the post's reactions, like state or comments, or `post_sel`. Sending the
  same post twice sends two messages. Messages are searched, so
  `shared a post` then finds the recipient's conversation.
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
- **The comment box** is `Mode::CommentBox { comment }`, where `comment` is
  the comment selected in the post view when you pressed `c`. Esc goes back to
  `PostDetail { comment }`, so the same comment is selected again. `c` clears
  `comment_draft`, so every visit starts empty, and Esc clears it too.
- **Commenting** (`post_comment`) pushes a `Comment` onto the end of the
  selected post's `comments` (the newest goes last, as in the seed): your
  profile's name and headline, age 0, and the draft with surrounding
  whitespace trimmed (what's inside is kept as typed). Then it selects the new
  comment (`PostDetail { comment: len - 1 }`), clears the draft and toasts
  "Comment posted". It changes nothing else: not the reactions or like state,
  `post_sel`, or another tab. Comments are searched by body, so a commented
  word makes the comment a result (but not its post), until it's deleted.
- **Deleting a comment** (`delete_comment`, `x` in `PostDetail`) removes the
  selected comment only when its author is your profile's name, and toasts
  "Comment deleted". The selection keeps its index, clamped to the list, as
  removing an invitation does on My Network: the next comment slides under
  the cursor, or the cursor moves up when the last one goes, and an empty
  list leaves it at 0. Anyone else's comment, or a post with no comments,
  gets nothing, not even a toast. Reactions, like state, `post_sel` and the
  other posts' comments don't change.
- **Opening a conversation** (`open_conversation`) is shared by `Enter` on
  Messaging and a search jump: it marks `chat_sel`'s conversation read, sets
  `open_chat`, clears `reply` and focuses the reply box (`Mode::Reply`).
- **Text boxes:** a key is typed when it is `KeyCode::Char` without Ctrl or
  Alt. Shift is fine.

## Key bindings

**Global:** Ctrl-C quits from any mode, including the text boxes.

Everywhere except the composer, the reply box, the comment box, search and
the Send picker:

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
| Home | `c` | in the post view, focus the "Add a comment" box with an empty draft. In the feed it does nothing |
| Home | `x` | in the post view, delete the selected comment if it's yours: "Comment deleted". Otherwise, and in the feed, nothing |
| Home | `s` | open the Send picker for the selected post, from the feed or the post view |
| My Network | `a` | accept the selected invitation: removed, connections +1, "Invitation accepted" |
| My Network | `x` | ignore it: removed, connections unchanged, "Invitation ignored" |
| My Network | `c` | on a suggestion, Connect ↔ Pending: "Invitation sent" / "Invitation withdrawn" |
| Jobs | `s` | save / unsave: "Job saved" / "Job unsaved" |
| Jobs | `e` | Easy Apply: "Application sent", then "Already applied" with no change |
| Jobs | `J` / `K` | scroll the details down / up one line (Shift-j / Shift-k), stopping at the top and where the description ends on the last row |
| Messaging | `Enter` | open the selected conversation, mark it read, focus the reply box |

`a`/`x` on a suggestion and `c` on an invitation do nothing, not even a
toast. `s` does nothing on My Network, Messaging and Notifications. `J` and
`K` do nothing off Jobs, including in the post view. Opening Notifications by any route marks them all read.

**Composer:** every printable key, including `q`, `j`, `k`, `J`, `K`, `l`,
`1`-`5`, `a`, `x`, `c`, `s` and `e`, is typed. `Backspace` deletes. `Enter` publishes
and toasts "Post published", or toasts "Write something first" and stays open
when the draft is empty or only whitespace. `Esc` discards the draft. `Tab`
does nothing.

**Search:** a text box like the composer. Every printable key without Ctrl
or Alt is typed, including `q`, `j`, `k`, `J`, `K`, `l`, `p`, `a`, `x`, `c`,
`s`, `e`, `/` and `1`-`5`, and does nothing else (`J`/`K` on Jobs don't
scroll). `Backspace` deletes, and does nothing
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
| comment `c` of post `p` | Home, `post_sel = p`, the post view with `PostDetail { comment: c }`: the comment is selected and scrolled into view, and `j`/`k` carry on from it. Esc goes back to the feed on post `p` |
| conversation `i` | Messaging, `chat_sel = i`, opened as `Enter` on it opens it: `open_chat = Some(i)`, marked read (the badge drops), reply draft cleared, reply box focused |

`Enter` with no results, or an empty query, toasts "No results" and keeps
search open with the query. `Esc` closes it and discards the query. `Tab`,
`BackTab` and every Ctrl- or Alt-modified key do nothing (Ctrl-C still
quits).

**Comment box:** a text box like search. Every printable key without Ctrl or
Alt is typed, including `q`, `j`, `k`, `J`, `K`, `l`, `c`, `x`, `p`, `/` and
`1`-`5`, and does nothing else (no quit, comment move, like, delete, composer, search or
tab switch). `Backspace` deletes, and does nothing on an empty draft (the box
keeps focus). `Enter` posts the comment, selects it and toasts "Comment
posted", or toasts "Write something first" and keeps the box focused, with
the draft as it was, when the draft is empty or only whitespace. `Esc`
discards the draft and returns to the post view with the same comment
selected. `Tab`, `BackTab`, the arrows and every Ctrl- or Alt-modified key do
nothing (Ctrl-C still quits).

**Send picker:** a text box like search. Every printable key without Ctrl
or Alt is typed into the filter, including `q`, `j`, `k`, `J`, `K`, `l`, `c`,
`x`, `p`, `s`, `/` and `1`-`5`, and does nothing else (no quit, tab switch, move, like,
comment box, delete, composer or search). `Backspace` deletes, and does
nothing on an empty filter (the picker stays open). `↓`/`↑` move through the
recipients, stopping at both ends, and any change to the filter selects the
first recipient again. `Enter` sends the post to the selected recipient,
closes the picker and toasts "Post sent", or, with no matches, toasts "No
matches" and keeps the picker open with the filter as typed. `Esc` closes it
and discards the filter. `Tab`, `BackTab` and every Ctrl- or Alt-modified key
do nothing (Ctrl-C still quits).

**Reply box:** printable keys, `J` and `K` included, are typed the same way. `Enter` appends the
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
                (which fits the job details scroll to the screen before each frame)
    app.rs      state, key handling, badges, search, sending. No rendering, no I/O
    data.rs     seed profile, posts, news, people, jobs, conversations, notifications
    ui.rs       draw(frame, &App): all rendering, pure
```

**Terminal restore:** `ratatui::init()` enables raw mode, enters the alternate
screen and installs a panic hook that calls `ratatui::restore()`. `restore()`
doesn't show the cursor, so `main.rs` wraps that hook to also send
`cursor::Show`, and does the same after a normal quit. Both paths were checked
in tmux: after `q`, and after a temporarily injected panic, `stty` matched
its state from before launch, the alternate screen was off and the cursor was
visible. Both were checked again after the comment box was added, the panic
injected while the box had focus, and again after the Send picker was added,
the panic injected while the picker was open. After search learned comments
and messages, both were checked again in a 120×30 private tmux session:
`q` after jumping to a conversation, and a temporarily injected panic fired
by a key while search showed `typo`'s post, comment and message results.
Each time `stty -a` matched its output from before launch, the alternate
screen was off and the cursor was visible. The injected panic was then
removed.

After the job details learned to scroll, the binary was run in an 80×20
private tmux session: `3`, then `J` until ` ↓ more ` went (two presses, with
`to a non-engineer.` on the card's last row), two more `J` that changed
nothing, and a `K` that moved the text at once. Back at the bottom, the
window was resized to 120×30: the card showed from the top with no
indicators. After `q`, `stty -a` differed from before launch only in the new
rows and columns, the alternate screen was off and the cursor was visible.

## Testing

130 unit tests, run with `cargo test` (about 20 s):

- **`app.rs`** (64), on the seed data:
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
    locations match, and so does a comment (`Number 5 is the real secret`).
    News and blank queries don't.
  - Live results: an accepted invitation drops out, a published
    `Rust meetup at Brightkettle` is the first post result, and a connected
    suggestion is pending.
  - `Enter` on each kind of result lands on the right tab and selection; the
    post jump lands in the feed even from the post view. No results toasts
    "No results" and stays open. The selection clamps at both ends and goes
    back to the first result when the query changes. Jumping from
    Notifications clears its markers, Esc doesn't.
  - Comment box, opening: `c` in the feed changes nothing, `c` on My Network
    still toggles Connect/Pending, and `c` in the post view focuses the box
    with the third comment remembered. Esc discards the draft and selects the
    third comment again, and reopening starts empty.
  - Comment box typing: `q1jklcp/ 5` is typed, with no quit, tab switch,
    comment move, like, composer or search. Shift is fine. `Tab`, `BackTab`,
    the four arrows and Ctrl/Alt keys (including Ctrl-Enter, Alt-Backspace and
    Ctrl-Esc) change nothing. Backspace on an empty draft keeps focus. Ctrl-C
    quits.
  - Posting on Maren Holt's post: a fourth comment, last, by Jordan Vale with
    your headline, age 0 and the trimmed body (inner double space kept),
    selected, with "Comment posted". Reactions, like state, `post_sel`, the
    other tabs and the other posts' comments are unchanged, and `/` then Esc
    keeps the new comment selected. Empty and whitespace-only drafts toast
    "Write something first" and keep focus and the draft.
  - A comment on Lucia Brandt's post survives Esc, a trip to Jobs and back and
    reopening the post. `zebra` from `zebra crossing` finds that comment only,
    not the post.
  - Deleting: with three comments of yours after Maren Holt's three, `x` on
    the middle one leaves the third under the cursor, `x` on the last moves
    the cursor up, each with "Comment deleted", until only the seed is left.
    Then `x` on a seed comment does nothing. Reactions, like states,
    `post_sel` and the other posts' comments are unchanged. Deleting the only
    comment on Lucia Brandt's post empties it with the selection at 0, and a
    second `x` toasts nothing.
  - No seed comment is yours. `x` in the feed, on each of Maren Holt's seed
    comments and on a post with no comments changes nothing and toasts
    nothing. In the comment box, with your own comment selected, `x` is
    typed.
  - Send picker, opening: `s` opens it with an empty filter from the feed
    (second post selected) and from the post view (third comment selected).
    Esc puts back exactly the same snapshot, sends nothing, and reopening
    starts empty. `/` then Esc still keeps the comment. `s` on Jobs still
    saves and unsaves, on My Network, Messaging and Notifications it changes
    nothing, and it is typed in the composer, the comment box, the reply box
    and search (so the picker can't open from search).
  - Send picker typing: with your own comment selected in the post view,
    `q1jklcxps/` is typed, with no quit, tab switch, comment move, comment
    box, like, delete, composer or search. Shift is fine. `Tab`, `BackTab`
    and Ctrl/Alt keys (including Ctrl-Enter, Alt-Enter and Ctrl-Esc) change
    nothing and send nothing. Backspace on an empty filter keeps it open. In
    the feed `p` and `l` are typed too. Ctrl-C quits.
  - Filtering: an empty or blank filter lists all four conversations,
    `  MAREN ` gives exactly Maren Holt, `o` gives Maren Holt, Ravi Tenorio
    and Ines Okonkwo in that order, and `Quillfeather`, `recruiter` and
    `coffee chat` (a headline word, a headline word, a message) give nothing.
    `↓`/`↑` clamp at both ends, and typing or deleting selects the first
    recipient again.
  - Sending Maren Holt's post to Felix Abernathy (filter `felix`) from the
    post view, with Ines Okonkwo's thread open and half a reply: exactly
    `Shared a post by Maren Holt: I'm happy to share that our team cut the CI
    queue from 40 minutes to 6.`, from you at age 0, is appended to Felix's
    conversation and no other, with "Post sent". The tab, mode (second
    comment selected), every selection, `open_chat`, the reply draft, the
    conversations' order and `unread` flags, the badge (2), and every post's
    reactions, like state and comments are unchanged. A second send adds a
    second message, and `Shared a post` finds Felix's conversation once,
    showing the newer of the two. Sending from the feed returns to the feed
    with the same post.
  - Sending to Maren Holt, who is unread, keeps her unread and the badge at
    2. `Enter` with `zqzq` toasts "No matches" and keeps the filter. A post
    you published is sent as `Shared a post by Jordan Vale: …` with the
    trimmed text.
  - Comments and messages: `typo` gives exactly Felix Abernathy's post, Maren
    Holt's comment on it and Felix's conversation (`Post(6)`,
    `Comment { post: 6, comment: 0 }`,
    `Conversation { chat: 3, message: Some(2) }`); `caching` gives Hana
    Sorensen's comment and Maren Holt's conversation; `felix` gives his post
    and his conversation but neither of his comments; `thanks` gives Wendell
    Achterberg's post, then Ines Okonkwo (two matching messages, one result)
    and Felix Abernathy. `bright` is unchanged. `the` lists comments by post,
    then in post view order.
  - The chosen message: `thanks` picks Ines Okonkwo's message 1 (your reply,
    not her older match) and Felix Abernathy's message 0 (his only match, not
    his newest), and `felix`, a name-only match, picks his newest, message 2.
  - A commenter's name (`noor haddad` is only the invitation), comment and
    conversation headlines (`quillfeather`, `Technical Recruiter`), news
    (`standing desks`) and notifications (`impressions`) don't make comment
    or conversation results.
  - `Enter` on Maren Holt's `typo` comment, from the post view of another
    post, lands on Felix's post (`post_sel = 6`) with
    `PostDetail { comment: 0 }`, and Esc returns to the feed on his post.
    From Jobs, `roadmap` lands on Wendell Achterberg's third comment, and `k`
    moves to the second.
  - `Enter` on Ravi Tenorio's conversation (`agenda`, the third result)
    opens it on Messaging: `chat_sel` and `open_chat` 1, read, the badge from
    2 to 1, a stale reply draft cleared, the reply box focused (a typed reply
    sends), and every snapshot field the same as `Enter` on him in Messaging.
  - Jumping to a comment or a conversation from Notifications ends the visit.
  - Live: a comment you post is found (and not by your name), follows its
    post down the feed when you publish, and is gone once deleted. A reply to
    Ines makes her conversation match. After sharing Maren Holt's post to
    Felix with `s`, `shared a post` finds his conversation, and Enter opens
    that conversation.
  - Job details scroll, with `fit_job_details` standing in for the event
    loop and Shift held as a terminal sends it: `J` and `K` move one line
    and stop at both ends, changing nothing else (selections, tab, mode,
    jobs, posts, people, drafts, overlays, toast). `K` at the top changes
    nothing. On Home (feed and post view), My Network, Messaging (list and
    after closing a thread) and Notifications (also after a second `5`),
    `J`/`K` change nothing and the Jobs scroll is still there on `3`.
  - No dead keys: 14 `J` on a 4-line overflow stop at 4, and the first `K`
    gives 3. After scrolling to 6, a screen that grows to an overflow of 2
    pulls it to 2 and the first `K` gives 1; an overflow of 0 gives 0 and `J`
    stays there; shrinking keeps the offset.
  - `j`, `↓`, `k` and `↑` to another job start it at the top, while `k` on
    the first job and `j` on the last keep the scroll. A search jump to a
    job (`lisbon`) starts at the top from another job, from the same job
    and from Home; a jump to a post leaves the Jobs scroll alone.
  - The scroll survives `3` → `1` → `3`, `Tab` then `BackTab`, `/` with a
    query then Esc, and `s`, `s`, `e`, `e` (all four toasts), and `K` still
    moves it after.
  - `J` and `K` are typed in search on all five tabs (`JK`, with the
    snapshot and the scroll unchanged), the composer, the comment box, the
    Send picker from the feed and the post view, and the reply box.
- **`ui.rs`** (66), rendering into a `TestBackend`:
  - The helpers `age`, `thousands`, `count`, `wrap`, `hard_wrap`,
    `truncate` and `first_visible`, plus `width_of`, `head` and `tail`. The
    text helpers are also checked on double-width text.
  - At 120×30, Home shows all three columns, in order. At 90×30 the sidebar
    is gone, and at 70×30 the profile card is too. The exact thresholds: the
    sidebar shows at 100 but not 99, and the card at 80 but not 79.
  - Every size from 1×1 to 140×40 renders without panicking, for 22 states:
    Home, post view, comment box (empty and with text), post view after
    commenting, composer (empty and with text), My Network (invitation and
    suggestion selected), Jobs (with and without a toast, scrolled two lines,
    and scrolled past the end so it draws as the bottom at every size), Messaging, reply
    box, Notifications, search (with matches, empty, no match, and over
    the post view), and the Send picker (from the feed, and with `o` over the
    post view). One thread per state.
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
  - Job details, scrolling with `J` fitted to the screen before every key
    as `main.rs` does: at 80×20 the Senior Rust Engineer starts at the top
    with ` ↓ more ` ending the bottom border and no ` ↑ more `, the last row
    `You have shipped Rust in production, enjoy`. Two `J` put `to a
    non-engineer.` on the last row inside the card with ` ↑ more ` and a
    plain bottom border; three more `J` leave the screen identical and the
    first `K` shows `writing tests, and can explain a tricky bug` last with
    ` ↓ more ` back. One `J` at 60×20 shows both. At 120×30 no job shows
    either and `J` leaves the screen identical. Scrolled to the bottom at
    60×20, the next frame at 120×30 equals the unscrolled one, before and
    after `fit_job_details`; at 60×23 the end stays on the last row and the
    first `K` moves the rows down one.
  - The details card at the top, scrolled two lines and past the end, at
    every size from 1×1 to 140×40: the corners are always `╭╮╰╯`, ` ↑ more `
    shows exactly when lines are hidden above and it fits beside the whole
    title, ` ↓ more ` exactly when lines are hidden below and it fits, a
    one-row card shows neither, and at the bottom the last row inside is
    never blank.
  - The Jobs footer reads exactly
    `j/k move  s save  e easy apply  tab next  1-5 tabs  q quit  J/K scroll`
    at 80×24. At every narrower width it is that line cut on the right, and
    it starts with exactly what the footer without `J/K scroll` showed at
    that width.
    No other tab's footer has `J/K`. Home, My Network, Messaging and
    Notifications render the same at 120×30, 80×20 and 60×20 with the Jobs
    details scrolled as without.
  - My Network, Jobs and Messaging contents, including `◷ Pending`,
    `✔ Applied`, unread markers and the reply box.
  - Each of the eleven hint sets is at most 80 columns, and is what the footer
    shows at 80×24. The post view footer reads exactly
    `j/k comments  l like  c comment  x delete  s send  esc back  1-5 tabs  q quit`
    (79 columns with the padding) and the
    comment box's `type write  backspace delete  enter post  esc cancel`.
    Those hints and every comment box label are single width.
  - Comment box states, at 120×30, 80×24 and 60×20: `Press c to comment`
    (dim, gray border), `Add a comment…▏` and `Nice one▏` (light blue
    border), and back to `Press c to comment` after Esc. A draft of wide text
    four times wider than the box shows its end and fills the box to within one
    column.
  - Posting raises `3 comments` to `4 comments` in the post view and on the
    feed card, with the new comment selected. On Lucia Brandt's post, `No
    comments yet` and `0 comments` become the `Comments` heading, the comment
    and `1 comment`, also on the card.
  - Deleting your comment brings `4 comments` back to `3 comments` in the
    post view and on the feed card, shows "Comment deleted" and selects the
    last seed comment. Deleting the only comment on Lucia Brandt's post shows
    `No comments yet` and `0 comments` again. `x` on a seed comment leaves the
    count and shows no toast.
  - After three long comments (the last ending in wide text), the newest is
    selected and at 120×30 and 60×20 its author line, headline and every body
    line are on screen above the box, ending on the row right above it.
  - The post view at every size from 1×1 to 140×40 with the box unfocused,
    focused and empty, focused with a short draft and with a 3,000-character
    draft mixing `日本語` and ASCII, and with the long comments (unfocused and
    focused): whenever the box shows, every row of it starts and ends with
    its border and is exactly the box's width, by display width. At 120×30,
    80×24 and 60×20 the box shows in full in every one of those states, and the
    header keeps the logo, all five numbers and the badges at 140, 120, 80, 70
    and 60 columns.
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
  - Send picker at 120×30: the `Send` title, `To Type a name▏` with the
    placeholder dim, `Maren Holt · I'm happy to share…` cut with `…` to the
    row's exact width, the rule, all four recipients' two lines with the
    first selected (bar and background), then `To o▏` with three recipients
    and the bar on Ravi Tenorio after `↓`, `No matches` for `zqzq`, and the
    post row following the selected post. With filter `o`, the only bold
    cells inside the border are the first match in each name, `o` in Maren
    Holt and Ravi Tenorio and `O` in Ines Okonkwo (the cell keeps the name's
    case), each at that name's first match; an empty filter bolds nothing,
    `FELIX` bolds `Felix`, and `ra` over the post view bolds `Ra`. At 50×20
    Felix Abernathy's headline
    is cut with `…`. A filter of wide text four times wider than the row
    shows its end and fills the row to within one column, at 120×30, 80×24
    and 60×20.
  - On Messaging after sending Maren Holt's post to Felix Abernathy: his row
    shows `now` and `You: Shared a post by`, the badge is still `(2)`, and
    with his thread open the last message is `Jordan Vale · now` and the
    whole shared text, wrapped, at 120×30 and 60×20.
  - Over a published post of wide text (eight times `a日本語…`), search (empty,
    `日本語` matching that post, and `zqzq`) and the composer (empty, a short
    draft and a wide draft) at every size from 1×1 to 140×40: every row of
    the box starts and ends with its border and is exactly the box's width,
    by display width.
  - The Send picker at every size from 1×1 to 140×40 from the feed (empty
    filter, `o`, `zqzq`, a 3,000-character filter mixing `日本語` and ASCII),
    from the post view (`felix` and the long filter), and over a published
    post of wide text eight times `a日本語…` long. Plus that post sent to
    Maren Holt with her thread open on Messaging. Whenever the picker is
    drawn, every row of it starts and ends with its border and is exactly the
    box's width, by display width. The header keeps the logo, all five
    numbers and exactly the non-zero badges at 140, 120, 80, 70 and 60
    columns in every one of those states. The long filter shows its end, the
    wide post's row is cut to within one column of the row, and the sent wide
    post shows in full at the bottom of the thread.
  - The Home, post view and picker footers read exactly as listed in "Key
    bindings" at 80×24, are at most 80 columns, and they and every picker
    label are single width.
  - Comments and messages at 120×30: `typo` shows `Posts`, `Comments`,
    `Messages` with `Maren Holt · on Felix Abernathy's post` / `Welcome
    aboard. The typo had it coming.` and `Felix Abernathy` / `On to the next
    typo.`; `caching` shows both new kinds; with a comment and a reply added,
    `bright` has all five groups in order (the end scrolls into view); the
    new empty prompt.
  - The conversation row: `thanks` shows `You: Thanks Ines, I'll keep it in
    mind.` and Felix's only match rather than his newest message; `felix`
    (name only) shows the newest message with no bold on it; a newer matching
    reply takes over.
  - Bold: `typo` gives exactly three `typo` runs, none on the comment's first
    line. `you` after replying `See you there` bolds only the body's `you`,
    never `You: `; `thanks` bolds `thanks`, `Thanks`, `Thanks`. With your
    comment `Holt's team rocks`, `holt` bolds nothing on
    `Jordan Vale · on Maren Holt's post`.
  - Excerpts: a match at the end of a long comment and a long reply shows
    `…` (or `You: …`) and that end, starting on a word, at 120×30 and 60×20;
    with `İstanbul` in both, `stan` keeps the word whole and bolds exactly
    `stan`, at 120, 60 and 40 columns.
  - A read conversation's overlay cells equal an unread one's.
  - At 80×24, `Enter` on `roadmap` shows Wendell Achterberg's third comment
    with the `┃` bar, which isn't on screen with the first comment selected.
  - A published comment and a sent reply of 3,000 characters mixing `日本語`
    and ASCII, ending in `bright終了`: `bright` (all five groups, first and
    last selected), `終了` and `日本語` (CJK only, comment and message), over
    the post view, and over Felix's open thread, at every size from 1×1 to
    140×40: every row of the box starts and ends with its border and is
    exactly its width by display width. At 120×30 `終了` shows both excerpts
    with the match bold. The new labels are single width.

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

For commenting, these breaks were each caught and then restored: inserting the
comment at the front instead of the end fails 3 tests
(`posting_a_comment_appends_it_and_selects_it`,
`a_posted_comment_raises_the_count_everywhere`,
`the_newest_long_comment_shows_whole_above_the_box`); letting `l` like the
post while the box has focus fails
`the_comment_box_types_keys_instead_of_acting_on_them` and
`a_posted_comment_raises_the_count_everywhere` (its comment has an `l`);
taking the draft's tail by `char` count instead of display width fails
`a_long_comment_draft_shows_its_end` and
`the_post_view_stays_inside_its_borders_at_every_size`; scrolling by the
whole card's height instead of the rows above the box fails
`a_posted_comment_raises_the_count_everywhere` and
`the_newest_long_comment_shows_whole_above_the_box`; and making Esc select
the first comment fails `c_focuses_the_comment_box_only_in_the_post_view`.

For deleting: letting `x` delete anyone's comment fails 3 tests
(`x_deletes_your_comment_and_keeps_the_index`,
`x_leaves_seed_comments_and_empty_posts_alone`,
`deleting_a_comment_lowers_the_count_everywhere`); not clamping the selection
fails `x_deletes_your_comment_and_keeps_the_index` and
`deleting_a_comment_lowers_the_count_everywhere`; and letting `x` delete from
the comment box fails `x_is_typed_in_the_comment_box` and
`the_post_view_stays_inside_its_borders_at_every_size` (its long draft has
`x`s in it).

For sending, each break was caught and then restored: sending to the
conversation at the filtered index in the unfiltered list fails 3 tests
(`sending_maren_holts_post_to_felix`, `a_published_post_is_sent_under_your_name`,
`a_sent_post_shows_on_messaging`); letting `j` move the selection while the
picker is open fails `the_send_picker_types_keys_instead_of_acting_on_them`;
marking the recipient read fails
`sending_to_an_unread_conversation_keeps_it_unread`; taking the filter's end
by `char` count instead of display width fails
`a_long_send_filter_shows_its_end` and
`the_send_picker_stays_inside_its_border_at_every_size`; and not blanking a
wide character left of the box fails
`the_send_picker_stays_inside_its_border_at_every_size` (at 134×13 the wide
post's text under the box's left edge hid the `╰`, which is how that bug was
found).

For the other overlays, which had the same bug before the picker existed:
clearing search with a plain `Clear` again fails
`search_stays_inside_its_border_over_a_wide_post` (first at 134×13, the
bottom row missing its `╰`), and doing the same in the composer fails
`the_composer_stays_inside_its_border_over_a_wide_post` (first at 70×13).
Drawing every picker name bold, as it was at first, fails
`only_the_first_match_in_each_name_is_bold`, and so does not searching the
names (no bold at all). Leaving the selection alone on `Backspace` in
`FilterBox::on_key` fails both `selection_clamps_and_resets_when_the_query_changes`
(search) and `send_selection_clamps_and_resets_when_the_filter_changes`
(the picker), since they share it.

For searching comments and messages, each break was caught and then
restored: choosing the oldest matching message instead of the newest
(`position` for `rposition` in `App::search_hits`) fails 4
(`comments_and_conversations_are_found_in_order`,
`sending_maren_holts_post_to_felix`,
`a_conversation_shows_its_newest_matching_message` and
`bold_marks_only_the_match_in_comment_and_message_rows`); matching comment
authors fails 5 (`comments_and_conversations_are_found_in_order`,
`authors_headlines_news_and_notifications_dont_match`,
`comments_you_post_or_delete_are_found_or_not`,
`a_conversation_shows_its_newest_matching_message` and
`bold_marks_only_the_match_in_comment_and_message_rows`); landing
with the comment's index as `post_sel` fails
`enter_on_a_comment_opens_its_post_with_the_comment_selected`,
`comments_you_post_or_delete_are_found_or_not` and
`enter_on_a_comment_far_down_shows_it_selected_at_80x24`; opening the
conversation without marking it read fails
`enter_on_a_conversation_opens_it_like_messaging_does`; drawing `You: ` bold
fails `bold_marks_only_the_match_in_comment_and_message_rows`,
`excerpts_for_matches_deep_in_a_long_comment_and_message` and
`long_comments_and_replies_stay_inside_the_search_border_at_every_size`; and
searching conversation headlines fails
`authors_headlines_news_and_notifications_dont_match`.

For scrolling the job details, each break was caught and then restored:
letting `J` go past `job_scroll_max` (clamping only when drawing) fails 3
(`shift_j_and_k_scroll_the_details_one_line_and_stop_at_the_end`,
`the_first_k_after_overscrolling_moves_the_text` and
`the_first_k_after_the_screen_grows_moves_the_text`); not pulling the
scroll back in `fit_job_details` fails
`the_first_k_after_the_screen_grows_moves_the_text` and
`a_terminal_that_grows_shows_no_blank_rows`; not clamping in `render_jobs`
fails `a_terminal_that_grows_shows_no_blank_rows` and
`the_details_card_keeps_its_corners_at_every_size`; not resetting on `j`/`k`
fails `moving_through_the_jobs_starts_each_at_the_top`, and not resetting
on a search jump fails `a_search_jump_to_a_job_starts_at_the_top`; showing
` ↑ more ` whatever the width, or with no `─` between it and the title,
showing ` ↓ more ` whatever the width, or showing either on a one-row card
fails `the_details_card_keeps_its_corners_at_every_size`; and dropping the
hint, or putting it second (right after `j/k move`) instead of last, fails
`the_jobs_footer_shows_j_k_scroll`.

## Ideas

Repost, profile pages, mouse support.
