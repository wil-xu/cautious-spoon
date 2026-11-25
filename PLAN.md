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
  to a conversation, reposting it to the top of your feed, writing a post, accepting or ignoring invitations,
  sending connection requests, saving jobs and applying, narrowing the jobs
  to Remote, Hybrid, On-site or saved ones with a row of filter pills,
  reading and replying to messages, and clearing notifications. `/` searches people, jobs, posts,
  comments and messages and jumps to the result. `v` opens the profile of
  the person under the selection: who they are, what they've done and what
  they've posted, in one place. Every text box (the composer, the comment
  and reply boxes, search and the Send picker) has a cursor you move with
  `←`/`→`, `Home` and `End`, so a typo is fixed where it is, not by deleting
  back to it.
- One self-contained binary. No network, no files written at runtime and no
  real data: all content is hard-coded, fictional and safe for work.

## Non-goals

- Real LinkedIn data or accounts (would need auth and APIs, and raises privacy
  concerns).
- Persisting state between runs. Likes, reposts, posts, comments,
  connections, saved jobs and messages reset on restart.
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
footer, after Repost, which only changed the footer again, and after
profile pages, which swapped `1-5 tabs` for `v profile` in it):

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
 j/k move  l like  enter open  p post  r repost  s send  v profile  q quit
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
start or end of a string that fits in N columns. `around_cursor` picks the
part of a one-line box's text to show around the cursor (see "Text boxes"
below). `hard_wrap` splits typed text
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
  80 columns. **A hint names only a key that does something right now.**
  Most sets are fixed: Home, My Network, Messaging list and
  Notifications. Jobs' (`jobs_hints`, below), the post view's
  (`post_view_hints`), a profile's (`profile_hints`, below) and the five
  text boxes' (`text_box_hints`) are built from the current state. Jobs,
  with details too long for the card, reads
  `j/k move  s save  e easy apply  f filter  1-5 tabs  q quit  J/K scroll`
  (72 columns with the padding). Home reads
  `j/k move  l like  enter open  p post  r repost  s send  v profile  q quit`
  (75), the post view
  `j/k comments  l like  c comment  r repost  s send  v profile  esc back  q quit`
  (80) with someone else's comment selected, My Network
  `j/k move  a accept  x ignore  c connect  v profile  tab next  1-5 tabs  q quit`
  (80) and Messaging `j/k move  enter open  v profile  tab next  1-5 tabs  q quit`
  (61). To make room for `r repost`, Home dropped `tab next` and the post
  view `1-5 tabs`. To make room for `v profile` (11 columns), Home dropped
  `1-5 tabs` too (74 + 11 would be 85). Both keys still work there. My
  Network had exactly 11 columns to spare, so it gives nothing up.
  - **The Jobs footer** follows the filtered list. To make room for
    `f filter` (10 columns; with it the old set would be 82), Jobs dropped
    `tab next`, as Home did first. `Tab` still works, `1-5 tabs` names the
    numbers the header shows, and every other tab's footer still says
    `tab next`. Dropping `q quit` or cutting `e easy apply` short would have
    saved too little, and `J/K scroll` is the only cue that the details
    scroll. `f filter` goes right after `e easy apply`, so `J/K scroll`
    stays last. `j/k move` shows only with two shown jobs or more, as
    `j/k comments` needs two comments. `J/K scroll` shows only when the
    selected job's details overflow the card on the current screen
    (`job_scroll_max > 0`, which the event loop sets before every frame), so
    it is there whenever `J` or `K` would move the text and never otherwise.
    At 80×24 every seed job fits, so the seed reads
    `j/k move  s save  e easy apply  f filter  1-5 tabs  q quit` there, its
    `Saved` list `s save  e easy apply  f filter  1-5 tabs  q quit`, and at
    80×20 each gets `J/K scroll` back. An empty list, where `s`, `e`, `j`,
    `k`, `J` and `K` do nothing, reads `f filter  1-5 tabs  q quit`. (Before
    the filters, `J/K scroll` showed even where everything fit.)
  - **The post view's footer** follows the selected comment and the post:
    - `x delete` shows exactly when the selected comment is yours
      (`App::is_yours`, the same check `x` makes before deleting), so the
      hint is there whenever `x` would delete and never otherwise. With
      `v profile` there is room for only one of `x delete` and `r repost`
      (each 10 columns; all three would be 90), so `x delete` takes
      `r repost`'s place:
      `j/k comments  l like  c comment  x delete  s send  v profile  esc back  q quit`
      (80). `r` still reposts, and Repost is still spelled out on the card's
      action row just above. `x delete` wins because deleting has no other
      cue on screen, and you only see it with your own comment selected.
      Dropping `q quit` or shortening `j/k comments` would not have been
      enough on its own, and `esc back` is how you leave the post view.
    - `j/k comments` shows only when the post has two comments or more, since
      with one or none `j` and `k` don't move anything: Lucia Brandt's post
      reads `l like  c comment  r repost  s send  v profile  esc back  q quit`.
  - **The text boxes' footers** are fixed sets plus `←/→ cursor`, last,
    once the box holds text: in an empty box the arrows have nowhere to go,
    as `j/k comments` needs two comments. It says `cursor`, not `move`,
    because search and the Send picker already say `↑/↓ move` for their
    results. Going last, it is the first thing a narrow footer cuts, so a
    footer too narrow for it reads as it did before the cursor could move
    (the reason `J/K scroll` is last on Jobs). With text the composer reads
    `type write  backspace delete  enter publish  esc discard  ←/→ cursor`
    (70), the comment box
    `type write  backspace delete  enter post  esc cancel  ←/→ cursor` (66),
    the reply box
    `type write  backspace delete  enter send  esc back to list  ←/→ cursor`
    (72), search `type search  ↑/↓ move  enter open  esc close  ←/→ cursor`
    (58) and the Send picker
    `type filter  ↑/↓ move  enter send  esc cancel  ←/→ cursor` (59).
    `Home`, `End` and `Delete` aren't hinted: `home/end jump` would take 15
    columns more, which pushes the composer (85), the comment box (81) and
    the reply box (87) past 80, and the five boxes read alike this way.
  - A narrower terminal cuts every set on the
  right. `J/K scroll` comes last so that a Jobs footer narrower than 72
  columns shows the set without it: everything up to `q quit` from 60
  columns, cut in the same place below that.
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
- **Text boxes** (the composer, the comment box, the reply box, search and
  the Send picker, and the top bar's search box while search is open) draw
  a light blue `▏` at the cursor (`TextField::cursor`, see "App state"). It
  goes between two characters, never over one, so it takes a column of its
  own, the column each box always kept for it. An empty box shows the
  cursor and its placeholder as it always did.
  - **One-line boxes** (the top bar, search's input row, the Send filter,
    the comment box and the reply box) show the part of the text that
    `around_cursor` picks from the text, the cursor and the width, storing
    nothing: the end of the text (`tail`) while that keeps at least a third
    of the width before the cursor, or when the whole text fits; otherwise
    the cursor sits a third of the way in (nearer the left edge close to the
    start of the text, so with the cursor at the start the start shows), and
    the text after it fills the rest. So with the cursor at the end, where
    typing leaves it, every box shows exactly what it showed before the
    cursor could move. Going left from the end the cursor walks to a third
    of the way in, then the text scrolls under it; going right from the
    start, the reverse. The window never moves against the cursor.
  - **The composer** inserts the `▏` into the draft at the cursor and
    splits the result into rows with `hard_wrap` (`wrap_with_cursor`, which
    also finds the row holding the cursor). It shows the last rows that
    still include the cursor's row: the end of the draft while the cursor is
    in it, as before, otherwise the cursor's row at the top.
  - **The cursor always shows.** Wherever a box is drawn with at least one
    row and one column inside its border and padding, its `▏` is on
    screen, however small the terminal. Everything else in the box gives
    way first:
    - search and the Send picker split their input row off first
      (`input_first`), so when the overlay has room inside for only one row
      (on a 9-row terminal), that row is the query or the filter, not the
      rule or the post line;
    - in a row too narrow for `⌕ ` (under 3 columns) or `To ` (under 4),
      the label is cut (`head`) to leave the cursor its column;
    - the composer drops its blank line, then its `Jordan Vale · Post to
      Anyone` line, when its inside is too short for them and a draft row;
    - an open thread splits its 3-row reply box off first (`thread_areas`),
      so on a short thread the reply box stays whole and the name, rule and
      messages give way.

    At normal sizes (an overlay with 3 rows inside, a composer with 3, a
    thread with 6) every box is laid out exactly as before.

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
  - `Like  Comment  Repost  Send`, where Like becomes a blue `Liked` and,
    once you've reposted the post, Repost becomes a blue `Reposted`, both
    bold. Like (`l`), Comment (`c` in the post view), Repost (`r`) and Send
    (`s`) all work.
- **Reposts:** `r` puts a repost card at the top of the feed. It is the same
  post shown a second time, not a copy: one dim line,
  `Jordan Vale reposted this · now` (your profile name; `reposted_line`),
  then exactly the lines the original's card shows, as the post is now.
  When the line doesn't fit, your name is cut with `…` like the author
  line's (`Jorda… reposted this · now`), as long as at least one column of
  the name shows next to the whole ` reposted this · now`. Narrower than
  that, the line keeps the whole words that fit and ends in `…`
  (`Jordan Vale reposted…`, `Jordan Vale…`), never stopping on the lone `·`
  or partway through a word; only when not even `Jordan…` fits is the first
  word cut (`Jor…`). A like or a comment on
  either shows on both, and both say `Reposted`. A post you haven't reposted
  looks as it always did.

  Captured from the real binary in a 120×30 tmux pane, after `r` on Maren
  Holt's post (the selection stays on her own card, the second):

  ```
  ╭────────────────────────────────────────────────────────────────╮
  │ Jordan Vale reposted this · now                                │
  │ Maren Holt · 2h                                                │
  │ Staff Engineer at Quillfeather Systems                         │
  │ I'm happy to share that our team cut the CI queue from 40      │
  │ minutes to 6.                                                  │
  │ No heroics: we cached the dependency build, split the sl …more │
  │ 342 reactions · 3 comments                                     │
  │ Like  Comment  Reposted  Send                                  │
  ╰────────────────────────────────────────────────────────────────╯
  ╭────────────────────────────────────────────────────────────────╮
  │ Maren Holt · 2h                                                │
  │ Staff Engineer at Quillfeather Systems                         │
  │ I'm happy to share that our team cut the CI queue from 40      │
  │ minutes to 6.                                                  │
  │ No heroics: we cached the dependency build, split the sl …more │
  │ 342 reactions · 3 comments                                     │
  │ Like  Comment  Reposted  Send                                  │
  ╰────────────────────────────────────────────────────────────────╯
  ```
- **Right, "WorkedIn News"** (`SIDEBAR_WIDTH = 30`): "Top stories", each with
  its age and reader count.
- **Responsive:** the right sidebar shows only at 100 columns or more, and the
  profile card only at 80 or more. Below 80 only the feed is left.
- **Post view (Enter):** the post replaces the feed column. It shows the full
  body with newlines kept, the counts and action row, a rule, then the
  comments, oldest first. `j`/`k` select a comment (marked with a blue `┃`).
  A post with no comments says "No comments yet" instead of the `Comments`
  heading. `Enter` on a repost card opens its original's post view, the same
  screen as from the original's own card, with no `reposted this` line.
  - **"Add a comment" box:** always at the bottom of the Post card, inside its
    border (`post_detail_areas` splits the card's inside into the comments and
    a 3-row box). It is drawn like the Messaging reply box
    (`render_comment_box`): a dim `Press c to comment` until `c` focuses it,
    then the draft with a `▏` at the cursor, or the dim placeholder
    `Add a comment…` followed by the cursor when the draft is empty. A draft
    too wide for the box shows the part around the cursor (`around_cursor`,
    by display width; see "Text boxes" above), so the cursor stays in view:
    its end, and what you just typed, while you type at the end. Its border is light blue
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
  pressing `c` again to start another comment (re-captured after the cursor
  could move, which only added `←/→ cursor` to the footer):

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
   type write  backspace delete  enter post  esc cancel  ←/→ cursor
  ```

  Then `←` thirteen times moves only the cursor, and the box reads
  `│ Happy to pair on▏ the write-up                             │`.
- **Composer (p):** a "Create a post" box centered over the body. It shows
  `Jordan Vale · Post to Anyone`, then the draft with a `▏` at the cursor,
  or the cursor and the placeholder "What do you want to talk about?". The
  draft is split into rows by display width (`hard_wrap`) rather than
  word-wrapped, so every typed space shows. A draft taller than the box
  shows the last rows that still hold the cursor's row (see "Text boxes"
  above): its end while you type at the end. In a box too short for the
  name line, a blank line and a draft row, the blank line goes, then the
  name line, so the cursor stays on screen.
- **Send picker (s):** `s` in the feed or the post view opens a "Send" box
  over the body for the selected post, sized and placed like the search
  overlay (`search_area`). From the top (`render_send`):
  - **Filter row** (`send_input`): `To ` and the filter with a `▏` at the
    cursor, or the dim placeholder `Type a name` and the cursor when the
    filter is empty. A filter too wide for the row shows the part around the
    cursor (`around_cursor`, by display width), so the cursor stays visible. In a row too narrow for the placeholder, the
    placeholder is cut (`head`), and in a row under 4 columns so is `To `.
    The filter row is split off first (`input_first`), so when the box has
    room inside for only one row, it is this one.
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
  bold change, which plain text doesn't show; the text is unchanged; and
  again after the cursor could move, which only added `←/→ cursor` to the
  footer):

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
   type filter  ↑/↓ move  enter send  esc cancel  ←/→ cursor
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

A "Jobs for you" list (40%) and "Job details" (60%). Captured from the real
binary in a 120×30 tmux pane after `3` and `f`:

```
╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ in    ⌕ Search                                 1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ Jobs for you · Remote (2) ───────────────────╮╭ Job details ─────────────────────────────────────────────────────────╮
│  All  Remote  Hybrid  On-site  Saved         ││ Senior Rust Engineer                                                 │
│ ▌Senior Rust Engineer ★                      ││ Quillfeather Systems                                                 │
│ ▌Quillfeather Systems                        ││ Lisbon, Portugal · Remote                                            │
│ ▌Lisbon, Portugal · Remote                   ││ Posted 1d ago · 48 applicants                                        │
│  Developer Advocate                          ││                                                                      │
│  Brightkettle                                ││  Easy Apply    ★ Saved                                               │
│  Anywhere in Europe · Remote                 ││                                                                      │
│                                              ││ About the job                                                        │
│                                              ││ Join the platform team building the services behind Quillfeather's   │
│                                              ││ document editor. You'll own a Rust storage layer, review designs,    │
│                                              ││ and mentor two junior engineers.                                     │
│                                              ││                                                                      │
│                                              ││ You have shipped Rust in production, enjoy writing tests, and can    │
│                                              ││ explain a tricky bug to a non-engineer.                              │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
│                                              ││                                                                      │
╰──────────────────────────────────────────────╯╰──────────────────────────────────────────────────────────────────────╯
 j/k move  s save  e easy apply  f filter  1-5 tabs  q quit
```

(`Remote` is bold, light blue and underlined on the real screen.)

- **Filters:** one at a time, `All` (the default), `Remote`, `Hybrid`,
  `On-site` or `Saved` (`JobFilter`). `f` steps to the next, wrapping from
  `Saved` to `All`. The list shows the jobs that match, in seed order
  (`App::shown_jobs`): by `workplace`, or for `Saved` by `saved` as it is
  now. In the seed each workplace has two jobs and `Saved` one.
- **Filter row** (`filter_row`): the first row inside the list card,
  `All  Remote  Hybrid  On-site  Saved`, starting after the selection bar's
  column like a list row. The active pill is bold, light blue and
  underlined, the others dim. The job rows start below it
  (`jobs_list_areas`), and selection, scrolling and the `▌` bar work as
  before in the rows left.
  - **Narrow cards:** the whole row needs 36 columns inside the card's
    padding (a 40-column card, from a 99-column terminal). On a narrower
    card the row shows the run of whole pills that holds the active one and
    starts as early as possible, with a dim `…` on each side that leaves
    pills out. At 80 columns that is `All  Remote  Hybrid …` for the first
    three, `… Remote  Hybrid  On-site …` and `… Hybrid  On-site  Saved`.
    When not even the active pill fits with a `…`, it shows alone, then
    without the bar's column, so its label is whole whenever the inside of
    the card is as wide as it; only a card narrower than its label cuts it
    with `…`. Nothing runs past the border.
- **Title** (`jobs_title`): ` Jobs for you ` with `All`, otherwise also the
  filter and how many jobs it shows, ` Jobs for you · Remote (2) `. A card
  too narrow for that (under 30 columns for `On-site`) drops `Jobs for you`,
  ` Remote (2) `, and a card too narrow for that cuts it with `…`, so it
  never runs into the corners.
- **Empty list:** when nothing matches (the seed's `Saved` after unsaving
  the Senior Rust Engineer), the list says `No saved jobs`, or `No jobs
  match` for any other filter, no row is selected, and the details card
  says `Select a job`, centered, with no indicators.
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
  fits it shows from the top, `J`/`K` change nothing visible and the footer
  leaves `J/K scroll` out.
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
  - **Reset:** the card goes back to the top whenever the selected job
    changes: `j`/`k`/`↓`/`↑` to another job, `f` to a filter that hides the
    selected job, and `s` that unsaves it out of the `Saved` list. It also
    goes back on every search jump to a job, even to the one already
    selected. `j` on the last shown job and `k` on the first, and `f` to a
    filter that still shows the selected job, keep it. Switching tabs and
    coming back, `/` then Esc, and `s` and `e` otherwise keep it, like every
    other selection.
- **Selection follows the job:** `f` keeps the selected job if the new
  filter shows it, and otherwise selects its first job. `j`/`k` move through
  the shown jobs only, stopping at both ends. `s` under `Saved` unsaves the
  job ("Job unsaved") and it leaves the list at once; the selection keeps
  its row, clamped to the list, as removing an invitation does on My
  Network: the next job slides under the cursor, or the cursor moves up when
  the last one goes. Saving or unsaving under any other filter changes no
  list, so the selection stays. The profile card's `Saved jobs` follows as
  always.

### Messaging

A conversation list (34 columns, at most half the width) and the open thread.

- **List rows:** a blue `●` and bold name for unread, the age of the last
  message, and a preview of it (`You: …` when it's yours).
- **Thread:** blank until you open one ("Select a conversation and press enter
  to open it"). Then: name and headline, a rule, the messages (`Name · age`,
  then the indented body, newest at the bottom), and a 3-row "Reply" box
  (`messaging_areas` and `thread_areas` lay these out; the reply box is
  split off first, so on a thread under 6 rows inside it stays whole and the
  rows above give way). The reply
  box shows "Press enter to reply" when the list has focus, and the draft with
  a `▏` at the cursor (or the cursor and `Write a message…` when empty) when
  it has focus. A long reply shows the part around the cursor
  (`around_cursor`, by display width), so the cursor never runs off the box:
  as much of its end as fits while you type at the end.
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
`typo` (re-captured after the cursor could move, which only added
`←/→ cursor` to the footer):

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
 type search  ↑/↓ move  enter open  esc close  ←/→ cursor
```

With a query longer than the box and the cursor moved to its start with
`Home`, both boxes show the start (same pane):

```
│ in    ⌕ ▏Unpopular opinion: the best meeting   1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
│ Software Engineer │ ⌕ ▏Unpopular opinion: the best meeting is the one that turns into a doc, and │ersus-spaces truce │
```

- **Input row** (`search_input`): `⌕ `, the query with a `▏` at the cursor,
  and on the right the result count, `5 results` or `1 result`. There is no
  count for an empty query, and none next to `No results`. A query too wide
  for the box shows the part around the cursor (`around_cursor`, by display
  width), its end while you type at the end. The count goes first: it only
  shows while the query keeps at least `QUERY_MIN_WIDTH` (10) columns, or all
  of a shorter one, so `⌕ bright▏  5 results` needs 20 columns and at 19 it
  is just `⌕ bright▏`. In a row under 3 columns the icon is cut so the cursor
  keeps its column. Under the input row, a rule. The input row is split off
  first (`input_first`), so a box with room inside for one row shows it.
- **Results:** the empty prompt `Type to search people, jobs, posts,
  comments and messages`, or `No results`, or the results grouped under
  `People`, `Jobs`, `Posts`, `Comments` and `Messages` headings, in that
  order. A group with no results has no heading. Each result is two lines:
  - person: the name, then `headline · Invitation` (or `· Connect` /
    `· Pending` for a suggestion). The label always shows; the headline is cut
    with `…` to make room,
  - job: the title, then `company · location · Remote/Hybrid/On-site`.
    Search ignores the Jobs filter: every matching job is found,
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
  query around the cursor, by the same rule as the input row. `header_fit` doesn't look at search, so the
  header lays out the same with search open or closed.

### Profile

`v` opens the profile of the person under the selection (see "Key
bindings"). It replaces the body under the header, whichever tab it was
opened from, as LinkedIn's profile page does; the header and its active tab
stay as they were. Captured from the real binary in a 120×30 tmux pane,
after `v` on Maren Holt's post:

```
╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ in    ⌕ Search                                 1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)│
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ Maren Holt · 1st                                                                                            Message  │
│ Staff Engineer at Quillfeather Systems                                                                               │
│ Lisbon, Portugal · 38 mutual connections                                                                             │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ About ───────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ I make build systems fast and boring. Fifteen years of backend work, the last five on Quillfeather's developer       │
│ platform. Ask me about caching, flaky tests or sourdough.                                                            │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ Experience ──────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ Staff Engineer                                                                                                       │
│ Quillfeather Systems · 2021 - present                                                                                │
│ Senior Software Engineer                                                                                             │
│ Ferngrove Analytics · 2015 - 2021                                                                                    │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ Activity ────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ ▌I'm happy to share that our team cut the CI queue from 40 minutes to 6.                                             │
│ ▌342 reactions · 3 comments                                                                                          │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯







 enter open  m message  esc back  1-5 tabs  q quit
```

From the top (`profile_cards`, drawn by `render_profile_page`), each a
full-width card:

- **Top card** (no title, `profile_top_lines`): the name in bold and the
  degree, `Maren Holt · 1st`; the headline; then
  `location · N mutual connections` (singular for 1). The action goes on the
  right of the name row, as the buttons do on a My Network row and drawn by
  the same helpers: `Ignore   Accept` for a pending invitation
  (`invitation_buttons`), `+ Connect` or `◷ Pending` for a suggestion
  (`connect_button`), and a ` Message ` button, white on blue, for someone
  you have a conversation with. When the whole name, the degree and the
  action don't fit on one row with two columns between, the action gets a
  row of its own under the location. Narrower still, the name is cut with
  `…` and the degree kept (`Theo Vasq… · 2nd`). Someone with no action (you,
  an accepted or ignored invitation, a connection you've never messaged)
  shows none.
- **About:** the member's few sentences, word-wrapped.
- **Experience:** two lines per role, newest first: the title in bold, then
  `Company · 2021 - present` dim. The first role is the current one and
  matches the headline.
- **Activity:** their posts, newest first, two lines each: the post's first
  line (`first_line`) cut with `…`, then its live `N reactions · M comments`
  (`counts_text`, also behind the feed card's counts line). The selected post
  has the usual `▌` bar and background (`render_list`), and the card the
  light blue border. With no posts the card says `No recent posts` and has
  a gray border.

**Your own profile** (from `v` on a post you published, or a repost of it)
has the top card with your name, no degree, your headline and
`312 connections` (live, so accepting an invitation raises it), then only
Activity: no About, no Experience and no action.

**Short and narrow screens:** the top card is always first, at the top of
the body. The cards below it give way whole, About first, then Experience,
when they and Activity's first 4 rows (`ACTIVITY_MIN_HEIGHT`: its border
and one post) don't fit (`first_visible` with Activity as the selection),
so the selected post stays on screen; within Activity, `render_list`
scrolls to it. The last card shown is cut short at the bottom and keeps its
border, like the last feed card. Every member's profile shows all four
cards at 80×24; at 60×20 an invitation's profile drops About, and at 30×16
Experience too. Every line is cut or wrapped by display width (`truncate`,
`wrap`), so a published post of wide text stays inside the Activity card.

**Hints** (`profile_hints`) name only keys that do something on this
profile now. `j/k posts` shows only with two posts or more in Activity (with
one there's nothing to move to), and `enter open` only with at least one.
Then comes just the action the top card offers: `a accept  x ignore`
(invitation), `c connect` or `c withdraw` (suggestion, by its state),
`m message` (conversation), or nothing. Last comes
`esc back  1-5 tabs  q quit`, which always work. So Noor Haddad's profile
reads `a accept  x ignore  esc back  1-5 tabs  q quit`, Maren Holt's (one
post) `enter open  m message  esc back  1-5 tabs  q quit`, and an accepted
invitation's just `esc back  1-5 tabs  q quit`. The longest possible, an
invitation with two posts, is 71 columns. The seed never has that: no
invitation posts.

## Data model (`src/data.rs`)

```rust
struct Profile      { name, headline, connections: u32 }
struct Post         { id: PostId, author, headline, hours_ago: u32,
                      body: String, reactions: u32, comments: Vec<Comment>,
                      liked: bool }
struct PostId(u32)
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
struct Member       { name, headline, location, mutual: u32, about,
                      experience: Vec<Role>, connected: bool }
struct Role         { title, company, years }
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
- **`Post::id` never changes, and is set once, where the post is built.**
  The `post` helper takes it as its first argument, and `data::posts` hands
  each seed post the next id as it builds it (0 to 8, in feed order), so no
  `Post` ever exists with a placeholder id. `App::publish` gives a new post
  one more than the highest id. Feed cards name their post by id, because
  publishing inserts at the front of `posts` and shifts every index.
- **The comment count is `comments.len()`.** Unlike workit, there is no
  separate displayed total, so the card and the post view always agree,
  including right after you comment.
- **One `Member` per person, not per place.** `data::members()` holds a
  profile for every post author, invitation, suggestion and conversation
  partner (every commenter is one of those too): 16 people, each once, and
  never you. A person who posts, comments and messages you, like Maren Holt
  or Ines Okonkwo, has the one profile. Each copy of a headline elsewhere in
  the seed (posts, comments, invitations, suggestions, conversations) is
  the member's headline, the member's first role is `title at company` of
  that headline, and an invitation's or suggestion's `mutual` is the
  member's; a test holds all of this to the seed, so a profile can't
  contradict a post card.
- **`Member::connected` is the only stored degree.** It is `true` in the seed
  for the nine post authors (and so the four conversation partners) and
  `false` for the invitations and suggestions, and accepting an invitation
  sets it. Ignoring one leaves it `false`.
- **Seed:** you are "Jordan Vale" with 312 connections. 9 posts, 5 news items,
  3 invitations, 4 suggestions, 6 jobs (1 saved), 4 conversations (2 unread),
  5 notifications (3 unread) and 16 members, each with a real city, a few
  sentences of About and two roles. Companies (Quillfeather Systems,
  Brightkettle, Pinecrest Studio, Harborline, Tinderbox Robotics, Larkspur
  Labs, and on Experience cards a few more, such as Ferngrove Analytics and
  Copperline Telecom) and people are invented. Cities are real.

## App state (`src/app.rs`)

| Field | Meaning |
|---|---|
| `profile`, `posts`, `news`, `invitations`, `suggestions`, `jobs`, `conversations`, `notifications`, `members` | seed data, mutated in place. `posts` holds every post once, newest first. `members` holds one profile per person other than you |
| `feed: Vec<Card>` | the Home feed, top to bottom. A `Card { post: PostId, repost: bool }` is a post's own card or your repost of it. Every post has exactly one own card, in the order of `posts`, and reposts go in between |
| `tab: Tab` | `Home`, `Network`, `Jobs`, `Messaging`, `Notifications` |
| `mode: Mode` | `Browse`, `PostDetail { comment }`, `CommentBox { comment }`, `Composer`, `Reply` |
| `post_sel`, `network_sel`, `job_sel`, `chat_sel`, `notif_sel` | one selection per list, kept while you visit other tabs. `post_sel` indexes `feed`. `job_sel` indexes `jobs`, not the filtered list |
| `job_filter: JobFilter` | `All`, `Remote`, `Hybrid`, `OnSite` or `Saved`: which jobs the Jobs list shows. Not persisted, but kept like a selection |
| `job_scroll` | lines of the Jobs details card scrolled off the top, never more than `job_scroll_max`. Back to 0 when the selected job changes, kept like a selection otherwise |
| `job_scroll_max` | how far the selected job's details can scroll on the current screen, set by `fit_job_details` |
| `open_chat: Option<usize>` | the thread shown in Messaging. `None` until you press Enter |
| `draft`, `reply`, `comment_draft` | composer, reply box and comment box: each a `TextField`, the text and its cursor |
| `search: Option<Search>` | `Some { query, selected }` while search is open. `query` is a `TextField`, `selected` indexes `App::search_hits` |
| `send: Option<SendPicker>` | `Some { query, selected }` while the Send picker is open. `query` is the filter, a `TextField`, and `selected` indexes `App::send_recipients` |
| `profile_page: Option<ProfilePage>` | `Some { name, post }` while a profile is open over the current tab and mode. `name` is whose (a member's or yours), `post` indexes `App::activity(name)` |
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

- **The Jobs filter selects jobs, not rows.** `job_sel` indexes `jobs`, so
  the selection follows the job when the filter changes, and the shown list
  (`App::shown_jobs`) is computed when asked, never stored. The selected job
  is `App::selected_job()`: `job_sel` while `job_filter` shows it. Every key
  that changes the filter or what it shows keeps that true or empties the
  list: `f` (`set_job_filter`) keeps `job_sel` when the new filter shows it
  and otherwise moves it to the first shown job; `s` under `Saved`
  (`drop_unsaved_job`) moves it to the job that took its row, clamped; a
  search jump to a job the filter hides sets the filter back to `All`.
  Saving under another filter, applying and `J`/`K` change no list. With an
  empty list `job_sel` stays where it was but `selected_job()` is `None`, so
  nothing is selected: `s`, `e`, `j`, `k`, `J` and `K` do nothing, the next
  `f` selects the first job of its filter, and `ui::job_scroll_max` is 0.
  `job_scroll` goes back to 0 exactly when `selected_job()` changes
  (`reset_job_scroll_if_moved`), which is when `job_sel` changed before
  there were filters. Nothing outside the Jobs tab touches `job_filter` but
  that search jump: `set_tab`, search, the Send picker and a profile leave
  it alone.
- **Derived counts:** badges (`App::badge`) and saved jobs
  (`App::saved_jobs`) are computed when asked, never stored. So is whether a
  post is reposted (`App::reposted(id)`: is there a repost card for it).
- **A repost is a card, not a post.** It holds only its original's id, so
  there is nothing to copy and nothing to go stale: `App::post(id)` finds
  the post wherever publishing has moved it. `selected_post` (and
  `selected_post_mut`) go through the selected card to the post behind it,
  and everything that acts on "the selected post" uses them: `l`, `Enter`,
  `j`/`k` in the post view, posting and deleting comments, the Send picker's
  post row and `send_post`. So all of them act on the original from a
  repost card. With no reposts, `feed[i]` is `posts[i]`'s card, so
  `post_sel` means what it did before reposts existed.
- **Reposting** (`repost`, `r` on Home, from the feed or the post view)
  inserts `Card { post, repost: true }` at index 0 for the selected card's
  post, adds 1 to `post_sel` so the selection stays on the same card, and
  toasts "Reposted". `mode` is untouched, so the post view stays open with
  the same comment selected. If the post already has a repost card, `r`
  toasts "Already reposted" and changes nothing. A repost card's post is its
  original, so `r` on it gets that toast too, and a repost is never reposted
  itself. Your own posts can be reposted. Nothing undoes a repost.
- **Search results** are never stored either. `App::search_hits(query)`
  recomputes them from the current state each time, as `SearchHit` values
  (`Invitation(i)`, `Suggestion(i)`, `Job(i)`, `Post(i)`,
  `Comment { post, comment }`, `Conversation { chat, message }`) indexing
  the live lists. Posts and comments come from `posts`, not `feed`, so a
  reposted post and each of its comments are found once, as the original.
  A jump to either lands on the post's own card (`card_of`), never on a
  repost.
  The query is trimmed and lowercased (`search_needle`), and a field matches
  when its `to_lowercase()` contains that. Fields: invitation and suggestion
  name and headline; job title, company and location; post author, headline
  and body; comment body (not its author or headline); conversation name and
  every message body (not its headline). Notifications and news aren't
  searched. Jobs are searched whatever `job_filter` shows. The order is people (invitations, then suggestions), jobs, posts
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
- **One editor for every text box.** `TextField { text, cursor }` holds a
  box's text and its cursor, a byte index into `text` that is always on a
  `char` boundary, so no slice at it can panic. Its fields are private:
  `text()`, `cursor()` and `is_empty()` read it, `clear()` empties it with
  the cursor at 0, and `TextField::from(&str)` (used by tests) puts the
  cursor at the end. `TextField::on_key` does all the editing for all five
  boxes (see "Text boxes" in "Key bindings") and reports what happened as an
  `Edit`: `Changed` when the text changed, `Unchanged` when it didn't (the
  cursor may have moved, or the key did nothing), or the key it leaves to
  the box's owner, `Enter`, `Esc`, `Up` or `Down`. `on_composer_key`,
  `on_comment_key` and `on_reply_key` act on `Enter` and `Esc` and ignore
  the rest. A step is one `char`, so `日` is one step and `Backspace` or
  `Delete` removes it whole. (A `char` isn't always what you'd call a
  character: an emoji with a skin tone, or a letter with a combining accent,
  is two or more steps.)
- **One filter box for search and the picker.** `Search` and `SendPicker`
  are both `FilterBox { query, selected }`, with `query` a `TextField`.
  `FilterBox::on_key(key, len)` hands the key to `TextField::on_key`, then:
  `Changed` selects the first item again, while `Unchanged` (a cursor move,
  or `Backspace` at the start or `Delete` at the end) keeps the selection;
  `↑`/`↓` move, stopping at 0 and `len - 1`. It hands `Enter` and `Esc` back
  as a `BoxKey`, and `on_search_key` and `on_send_key` decide what they do
  (jump or send, close).
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
  first newline; a published post has none, so it's the whole post). From a
  repost card that is the original's author and body, exactly as from the
  original's card. Then it
  toasts "Post sent". It changes nothing else: not `unread` (so the badge
  stays), the order of the conversations, `chat_sel`, `open_chat`, `reply`,
  the post's reactions, like state or comments, or `post_sel`. Sending the
  same post twice sends two messages. Messages are searched, so
  `shared a post` then finds the recipient's conversation.
- **Switching tabs** (with `1`-`5`, `Tab` or `BackTab`) closes the post view
  and the profile page, so coming back to Home shows the feed with the same
  post selected.
  Entering Notifications from another tab copies each `unread` into `is_new`
  and clears `unread`. Leaving it clears `is_new`. Pressing `5` while already
  there changes nothing.
- **Removing an invitation** keeps `network_sel` and clamps it to the list. The
  next row slides under the cursor, or the cursor moves up when the last row
  goes. An empty list leaves it at 0, where `a`, `x` and `c` do nothing.
- **Publishing** inserts the post at index 0 of `posts`, with a new id, and
  its card at index 0 of `feed`, and sets `post_sel` to 0. The published body
  is the draft with surrounding whitespace trimmed.
- **The comment box** is `Mode::CommentBox { comment }`, where `comment` is
  the comment selected in the post view when you pressed `c`. Esc goes back to
  `PostDetail { comment }`, so the same comment is selected again. `c` clears
  `comment_draft` (text and cursor), so every visit starts empty with the
  cursor at 0, and Esc clears it too.
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
- **A profile sits on top of `mode`,** like search. `v` (or `Enter` on My
  Network) sets `profile_page` and leaves `tab`, `mode` and every selection
  alone, so Esc (`profile_page = None`) puts back exactly what was there,
  including the post view's selected comment and the open thread. While it
  is open, `on_key` sends keys to `on_profile_key` (after search and the
  picker, before the mode), so nothing reaches the tab underneath. `/`
  opens search over it, and search's Esc returns to it. Switching tabs
  (`set_tab`) closes it, which also covers every search jump, `m` and
  `Enter` on Activity.
- **Nothing on a profile is stored but whose it is and the selected post.**
  `App::degree(name)` reads `Member::connected` (`1st` or `2nd`; `None` for
  you). `App::profile_action(name)` looks the name up in the live lists:
  `Invitation(i)` while they're in `invitations`, `Suggestion(i)` while in
  `suggestions` (`+ Connect` or `◷ Pending` from its `pending`), otherwise
  `Message(chat)` when you have a conversation with them, and nothing for
  you. `App::activity(name)` is the indexes of their posts in `posts`, which
  is newest first, so a post you publish goes first in your Activity at
  once. Counts are read from the posts when drawn.
- **Accepting, ignoring and connecting share one function.**
  `act_on_network_item(code, item)` does what `a`, `x` and `c` do, with the
  same toasts, then clamps `network_sel` as removing an invitation always
  has. My Network calls it with the selected row, a profile with the
  person's own row (`network_item_of(name)`), or `None`, which does nothing
  but the clamp. Accepting also sets `connected`, so the open profile shows
  `1st` and no action at once. A profile opened from My Network is the
  selected row's, so after `a` or `x` there and Esc, the list shows the same
  selection as `a` or `x` on the row would.
- **The routes out are shared too.** `go_to_post(i, mode)` (Home, on post
  `i`'s own card via `card_of`, then `mode`) is what search's post and
  comment jumps and `Enter` on Activity (`PostDetail { comment: 0 }`) use.
  `go_to_conversation(chat)` (Messaging, `chat_sel = chat`, then
  `open_conversation`) is what search's conversation jump and `m` use.
- **Text boxes:** a key is typed when it is `KeyCode::Char` without Ctrl or
  Alt. Shift is fine. Every place that empties a box (`Esc`, publishing,
  posting a comment, sending a reply, `c`, `open_conversation`, and opening
  search or the picker, which start from `FilterBox::default()`) leaves it
  with empty text and the cursor at 0.

## Key bindings

**Global:** Ctrl-C quits from any mode, including the text boxes.

Everywhere except the composer, the reply box, the comment box, search and
the Send picker:

| Key | Action |
|---|---|
| `1`-`5` | switch tab (from a profile too, closing it) |
| `Tab` / `Shift-Tab` | next / previous tab, wrapping both ways |
| `j`/`↓`, `k`/`↑` | move the selection in the current list (comments in the post view, Activity on a profile) |
| `/` | open search (also from the post view and a profile) |
| `Esc` | close the profile, or else the post view. Otherwise nothing, so it never quits |
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
| Home | `r` | repost the selected post to the top of the feed: "Reposted", the selection stays on the same post (and comment, in the post view). "Already reposted" and no change if it is reposted already, including on its repost card |
| Home | `v` | open the profile of the selected post's author, from the feed or the post view (always the post's author there, whichever comment is selected). On a repost card, the original's author. On a post you published, your own profile |
| My Network | `a` | accept the selected invitation: removed, connections +1, "Invitation accepted" |
| My Network | `x` | ignore it: removed, connections unchanged, "Invitation ignored" |
| My Network | `c` | on a suggestion, Connect ↔ Pending: "Invitation sent" / "Invitation withdrawn" |
| My Network | `v` / `Enter` | open the selected invitation's or suggestion's profile |
| Jobs | `s` | save / unsave: "Job saved" / "Job unsaved". Under `Saved`, an unsaved job leaves the list and the next one (or the new last) is selected |
| Jobs | `e` | Easy Apply: "Application sent", then "Already applied" with no change |
| Jobs | `J` / `K` | scroll the details down / up one line (Shift-j / Shift-k), stopping at the top and where the description ends on the last row |
| Jobs | `f` | next filter: All → Remote → Hybrid → On-site → Saved → All. The selected job stays if the new filter shows it, otherwise its first job is selected. Also on an empty list |
| Messaging | `Enter` | open the selected conversation, mark it read, focus the reply box |
| Messaging | `v` | open the profile of the selected conversation's person (the one under the cursor, not the open thread), from the list |

`v` does nothing on Jobs and Notifications, and on a profile. `f` does
nothing on Home (feed and post view), My Network, Messaging, Notifications
and a profile, not even a toast. On Jobs with an empty filtered list, `s`,
`e`, `j`, `k`, `J` and `K` do nothing, not even a toast; `f` still works.

**Profile:** keys go to the profile, never to the tab and mode under it, so
`l`, `r`, `s`, `p`, `e`, `f`, `J`, `K` and `v` do nothing there.

| Key | Action |
|---|---|
| `j`/`↓`, `k`/`↑` | move through Activity, stopping at both ends |
| `Enter` | go to the selected Activity post: Home, on the post's own card (never a repost), in the post view with its first comment selected, the route a search jump to a comment takes. Nothing with no Activity |
| `a` / `x` | accept / ignore their pending invitation, exactly as on My Network (same toasts, connections, badge and clamped `network_sel`). The profile stays open: accepted, it shows `1st` and no action; ignored, `2nd` and no action |
| `c` | connect to / withdraw from their suggestion, as on My Network. The button follows at once |
| `m` | open your conversation with them, as `Enter` on Messaging does (`open_conversation`: Messaging, marked read, reply draft cleared, reply box focused). Nothing without a conversation |
| `Esc` | close the profile, back to exactly the tab, mode and selections it was opened from |
| `/` | search over the profile. Esc from search returns to it; a jump closes it |
| `1`-`5`, `Tab`, `Shift-Tab` | switch tab, closing the profile, as from the post view |
| `q` | quit |

`a`, `x` and `c` do nothing, not even a toast, when the person isn't a
pending invitation (`a`, `x`) or a suggestion (`c`): on a connection's
profile, your own, or after accepting or ignoring.

`a`/`x` on a suggestion and `c` on an invitation do nothing, not even a
toast. `s` does nothing on My Network, Messaging and Notifications. `r` does
nothing on My Network, Jobs, Messaging and Notifications. On a repost card,
`l`, `Enter`, `s` and, in its post view, `c` and `x` act on the original. `J` and
`K` do nothing off Jobs, including in the post view. Opening Notifications by any route marks them all read.

**Text boxes:** the composer, the comment box, the reply box, search and the
Send picker edit their text the same way (`TextField::on_key`):

| Key | Action |
|---|---|
| a printable key | typed at the cursor, which moves past it. Shift is fine |
| `←` / `→` | move the cursor one character, stopping at both ends. A character is a `char`, so `日` is one step and never split |
| `Home` / `End` | move the cursor to the start / end |
| `Backspace` | delete the character before the cursor. Nothing at the start |
| `Delete` | delete the character under the cursor. Nothing at the end |

Every Ctrl- or Alt-modified key does nothing in a text box, the arrows,
`Home`, `End`, `Delete`, `Backspace`, `Enter` and `Esc` included, and
Ctrl-C still quits. (Before the boxes shared `TextField`, the composer and
the reply box let a modified `Enter`, `Esc` or `Backspace` through; now they
are like the other three.) `Tab` and `BackTab` do nothing. Every box opens
empty with the cursor at 0, and is that again whenever it is emptied.
Outside the text boxes `←`, `→`, `Home`, `End` and `Delete` do nothing, as
before.

**Composer:** every printable key, including `q`, `j`, `k`, `J`, `K`, `l`,
`1`-`5`, `a`, `x`, `c`, `s`, `e`, `f`, `r`, `v` and `m`, is typed, and the text
edits as above. `Enter` publishes the draft as edited
and toasts "Post published", or toasts "Write something first" and stays open
when the draft is empty or only whitespace. `Esc` discards the draft. `↑`,
`↓`, `Tab` and `BackTab` do nothing.

**Search:** a text box like the composer. Every printable key without Ctrl
or Alt is typed, including `q`, `j`, `k`, `J`, `K`, `l`, `p`, `a`, `x`, `c`,
`s`, `e`, `f`, `r`, `v`, `m`, `/` and `1`-`5`, and does nothing else (`J`/`K` on Jobs don't
scroll, and `f` doesn't filter). The query edits as in "Text boxes" above; `Backspace` at the
start, on an empty query too, does nothing (search stays open). `↓`/`↑`
move through the results, stopping at both ends. Any change to the query
selects the first result again; only a real change counts, so moving the
cursor, `Backspace` at the start and `Delete` at the end keep the selected
result. `Enter` closes search and goes to the selected result, switching tabs
the way `1`-`5` do (so leaving Notifications ends the visit):

| Result | Lands on |
|---|---|
| invitation `i` | My Network, `network_sel = i` |
| suggestion `i` | My Network, `network_sel = invitations + i` |
| job `i` | Jobs, `job_sel = i`, the details at the top. If `job_filter` hides job `i`, it goes back to `All`; if it shows it, it stays |
| post `i` | Home feed (not the post view), `post_sel` on post `i`'s own card (`i` itself until you repost) |
| comment `c` of post `p` | Home, `post_sel` on post `p`'s own card, the post view with `PostDetail { comment: c }`: the comment is selected and scrolled into view, and `j`/`k` carry on from it. Esc goes back to the feed on post `p` |
| conversation `i` | Messaging, `chat_sel = i`, opened as `Enter` on it opens it: `open_chat = Some(i)`, marked read (the badge drops), reply draft cleared, reply box focused |

`Enter` with no results, or an empty query, toasts "No results" and keeps
search open with the query. `Esc` closes it and discards the query. `Tab`,
`BackTab` and every Ctrl- or Alt-modified key, the arrows included, do
nothing (Ctrl-C still quits).

**Comment box:** a text box like search. Every printable key without Ctrl or
Alt is typed, including `q`, `j`, `k`, `J`, `K`, `l`, `c`, `x`, `p`, `f`, `r`,
`v`, `m`, `/` and `1`-`5`, and does nothing else (no quit, comment move, like, delete,
repost, composer, search or tab switch). The draft edits as in "Text boxes"
above, and `←`/`→` move the cursor, not the selected comment; `Backspace` at
the start, on an empty draft too, does nothing (the box keeps focus).
`Enter` posts the comment as edited, selects it and toasts "Comment
posted", or toasts "Write something first" and keeps the box focused, with
the draft as it was, when the draft is empty or only whitespace. `Esc`
discards the draft and returns to the post view with the same comment
selected. `Tab`, `BackTab`, `↑`/`↓` and every Ctrl- or Alt-modified key do
nothing (Ctrl-C still quits).

**Send picker:** a text box like search. Every printable key without Ctrl
or Alt is typed into the filter, including `q`, `j`, `k`, `J`, `K`, `l`, `c`,
`x`, `p`, `s`, `f`, `r`, `v`, `m`, `/` and `1`-`5`, and does nothing else (no
quit, tab switch, move, like, comment box, delete, repost, composer or search). The filter edits as in
"Text boxes" above; `Backspace` at the start, on an empty filter too, does
nothing (the picker stays open). `↓`/`↑` move through the recipients,
stopping at both ends, and any change to the filter selects the first
recipient again, while moving the cursor, `Backspace` at the start and
`Delete` at the end keep it. `Enter` sends the post to the selected recipient,
closes the picker and toasts "Post sent", or, with no matches, toasts "No
matches" and keeps the picker open with the filter as typed. `Esc` closes it
and discards the filter. `Tab`, `BackTab` and every Ctrl- or Alt-modified key,
the arrows included, do nothing (Ctrl-C still quits).

**Reply box:** printable keys, `J`, `K`, `f`, `r`, `v` and `m` included, are typed the same way, and the
reply edits as in "Text boxes" above. `Enter` appends the reply as edited
(trimmed) to the thread as yours, clears the box and keeps focus. An empty
or whitespace-only reply does nothing. `Esc` discards the text and returns
focus to the list. `↑`, `↓`, `Tab` and `BackTab` do nothing.

## Files

```
workedin/
  PLAN.md
  Cargo.toml    ratatui 0.30 only, edition 2024
  Cargo.lock
  src/
    main.rs     ratatui::init()/restore() plus cursor::Show, the panic hook, the event loop
                (which fits the job details scroll to the screen before each frame)
    app.rs      state, key handling, badges, search, sending, profiles. No rendering, no I/O
    data.rs     seed profile, posts, news, people, jobs, conversations, notifications,
                and one member profile per person
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

After Repost was added, the binary was run in a 120×30 private tmux session:
`r` on Maren Holt's post put the repost card first, with a gray border,
while her own card below kept the light blue one, and the footer showed
"Reposted". `l` made both cards `343 reactions` and `Liked`. `k`, `Enter`,
`c`, a comment and `Enter` showed `4 comments` in the post view, and after
`Esc` both cards showed `4 comments`, with the repost card selected.
Publishing `hello` put it first and the repost below it still showed Maren
Holt's post. After `q`, `stty -a` matched its output from before launch,
the alternate screen was off and the cursor was visible.

After profile pages were added, both paths were checked again in a 120×30
private tmux session (`tmux -L wprof`): `q` from Baptiste Moreau's profile
(opened with `2`, `j`, `v`, showing `Ignore   Accept`), and a temporarily
injected panic fired by a key on Maren Holt's profile, after `j`. Each time
`stty -a` matched its output from before launch, `#{alternate_on}` was 0
and `#{cursor_flag}` 1, and after the panic its message was printed on the
normal screen. The injected panic was then removed.

## Testing

210 unit tests, run with `cargo test` (about 25 s):

- **`app.rs`** (109), on the seed data:
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
    comment move, like, composer or search. Shift is fine. `←` and `→` move
    the cursor and nothing else (the same comment stays selected). `Tab`,
    `BackTab`, `↑`, `↓` and Ctrl/Alt keys (including Ctrl-Enter,
    Alt-Backspace, Ctrl-Esc, Ctrl-`←` and Alt-`Home`) change nothing.
    Backspace on an empty draft keeps focus. Ctrl-C quits.
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
  - Reposting from the feed (Tobias Ferreira-Lund's post): a repost card
    for his post goes first, "Reposted", the selection moves to card 2, his
    own card, and `posts` doesn't grow. From Maren Holt's post view with the
    third comment selected: the same comment stays selected, `k` carries on
    from it, and Esc lands on her own card.
  - `r` on My Network, Jobs, Messaging and Notifications changes nothing:
    the snapshot, the feed, every post's reactions, like state and comments,
    jobs, invitations and connections, and no toast.
  - A second `r` on the original, on the repost and in the repost's post
    view each toast "Already reposted" and change nothing else, leaving one
    repost. Your own published post can be reposted.
  - From the repost: `l` likes the original (343, liked) and `l` on the
    original unlikes it. A comment posted from the repost's post view is the
    original's fourth, selected, and shows in the original's post view too;
    Esc returns to the repost card. Deleting it from the repost's post view
    brings back 3 and moves the cursor up. Other posts are untouched.
  - Reposting Maren Holt's post, publishing `hello`, then reposting Lucia
    Brandt's gives exactly `+Lucia Brandt, Jordan Vale, +Maren Holt, Maren
    Holt, …, Lucia Brandt, Wendell Achterberg`, and each repost's id is its
    original's.
  - With Felix Abernathy's post reposted, `typo` still gives exactly
    `Post(6)`, `Comment { post: 6, comment: 0 }` and his conversation. The
    post jump lands on his own card (7, not the repost), and so does the
    comment jump from the repost's post view, with `PostDetail { comment: 0 }`.
  - `s` on a repost, from the feed and from its post view, sends exactly
    what `s` on Maren Holt's own card sends.
  - `r` is typed in search on all five tabs and over the post view (query
    `r`, nothing else changed), the composer, the comment box, the Send
    picker from the feed and the post view, and the reply box, and never
    reposts.
  - Post ids are unique across the 9 seed posts and still after publishing
    two more.
  - `assert_feed_matches_posts` checks that `feed` and `posts` agree: every
    post has exactly one card of its own, those cards come in `posts` order,
    and every repost card names a post in `posts`, at most one per post. It
    runs at the end of every repost test (on every app in the ones that
    build several), after publishing in
    `reposts_keep_their_original_when_cards_go_in_above`, and in the id test.
  - Profiles, with `world` (the snapshot plus the feed, every post's
    reactions, like state and comments, jobs, invitations, suggestions,
    connections, conversations and the three drafts) as the "nothing else
    changed" check:
    - Every entry point: `v` on the feed (Maren Holt, then Tobias
      Ferreira-Lund), in the post view with the third comment selected (the
      post's author, the comment still selected), on invitations and
      suggestions with `v` and with `Enter`, and on Messaging (the
      conversation under the cursor, with another thread open, which stays
      open and nothing is marked read). Opening changes nothing in `world`.
      `v` on Jobs and Notifications changes nothing and toasts nothing.
    - On a repost card, from the feed and from its post view, `v` opens the
      original's author; on a repost of your own post, your profile.
    - Esc puts back exactly the `world` it was opened from, after `j`, `k`,
      an unknown key and a search opened, typed and closed (which returns
      to the profile): from the feed, the post view (third comment), a
      repost card, My Network and Messaging with an open thread.
    - On a profile `l`, `r`, `s`, `p`, `c`, `x`, `a`, `e`, `v`, `J`, `K` and
      Backspace on a connection's profile change nothing and toast nothing.
      Ctrl-C and `q` quit.
    - `1`-`5`, `Tab` and `BackTab` close it, landing in Browse on the new
      tab, and `1` then shows the feed on the same post. A search jump from
      a profile goes to the result and closes it.
    - Accepting Baptiste Moreau on his profile: "Invitation accepted",
      313 connections, badge 2, the profile still open, now `1st` with no
      action; then `a`, `x`, `c` and `m` do nothing. After Esc, `world`
      equals `a` on his row in My Network, selection included. Ignoring
      Noor Haddad: "Invitation ignored", 312, `2nd`, no action, and a second
      `a` does nothing.
    - With no suggestions, `a` and `x` on the last invitation's profile
      clamp `network_sel` to 1 at once, and after Esc `world` equals the
      same key on the row. With suggestions, the first one slides under the
      cursor.
    - `c` on Amara Lindgren's profile: "Invitation sent", pending, still
      `2nd`, then "Invitation withdrawn". `a`, `x` and `m` on a suggestion,
      `c` on an invitation and `a`, `x`, `c` on a connection do nothing.
    - `m` on Ravi Tenorio's profile from the feed: Messaging, `chat_sel` and
      `open_chat` 1, read, badge 1, `Reply`, the stale draft cleared, and
      `world` equal to `Enter` on him in Messaging. From Messaging it opens
      Felix Abernathy's thread over Maren Holt's. On Tobias Ferreira-Lund
      (a connection without a conversation) and Saoirse Kimura it does
      nothing.
    - `Enter` on Activity: from Messaging, Ines Okonkwo's post opens in the
      post view on card 2 with comment 0, Esc returns to the feed on it, and
      it lands exactly where search's jump to her post's comment lands. With
      Hana Sorensen's post reposted, `Enter` from the repost's profile lands
      on her own card (6). With no Activity it does nothing.
    - Your own profile after publishing two posts: Activity `[0, 1]`, newest
      first, no degree, no action, no member; `j`/`↓` stop at 1, `k`/`↑` at
      0; `a`, `x`, `c` and `m` do nothing; `Enter` on the older post opens
      it. Seed authors have one post each, invitations none.
    - `v` and `m` are typed in the composer, the comment box, the reply box,
      search on every tab, over the post view and over a profile, and the
      Send picker from the feed and the post view, and open nothing.
    - One profile per person, with the headline shown everywhere: every
      post author, commenter, invitation, suggestion and conversation (you
      excepted, whose headline is the profile's) has exactly one member,
      with the same headline; every member is someone the app shows, there
      are 16, the first role is `title at company` of the headline and
      `present`, there is an earlier role, a `City, Region` location and an
      About of more than one sentence. Invitations' and suggestions'
      `mutual` match their member's and they are `2nd`; post authors and
      conversation partners are `1st`.
  - Text boxes:
    - `TextField` on its own: typing after `←` inserts before the last
      character with the cursor past it, `Home` then typing inserts at the
      start, `Backspace` and `Delete` in the middle each remove one
      character, and `Backspace`/`←` at the start and `Delete`/`→` at the
      end change nothing and report `Unchanged`. `Enter`, `Esc`, `↑` and
      `↓` come back to the owner; `Tab` and `BackTab` do nothing. With Ctrl
      or Alt, the arrows, `Home`, `End`, `Delete`, `Backspace`, a letter,
      `Enter`, `Esc` and `↑` do nothing. Shift-`X` is typed.
    - `日` is one step each way and deleted whole by `Delete` and by
      `Backspace`. From every cursor position in a text of 1- to 4-byte
      characters, every editing key leaves the cursor on a `char` boundary
      without panicking.
    - In each of the five boxes, through `App::on_key`: `←` then typing
      inserts in the middle, `Home` then typing at the start, `End` goes
      back, `Backspace` and `Delete` in the middle, `Backspace` and `←` at
      the start and `Delete` and `→` at the end changing nothing, `日`
      stepped over in one move and deleted whole, and Ctrl/Alt-modified
      arrows, `Home`, `End`, `Delete` and `Backspace`, `Tab` and `BackTab`
      changing nothing. The tab, mode and every selection stay put, with
      no toast. In the composer, comment box and reply box `↑`/`↓` change
      nothing.
    - Edited in the middle, then submitted: the composer publishes
      `Hello world` (typed `Helo world`), the comment box posts
      `Great post`, the reply box sends `See you there`, search opens
      exactly where typing `lisbon` straight would (typed `lisbn`, `←`,
      `o`), and the Send picker sends to Felix Abernathy (typed `flix`).
    - Search (`bright`) and the picker (`o`): with the third result
      selected, `←`, `→`, `Home`, `Backspace` at the start, `End`, `Delete`
      at the end and `→` keep it; typing in the middle, `Delete` at the
      start and a `Backspace` that deletes select the first again.
    - Every box is empty with the cursor at 0 when it opens, after `Esc`,
      after publishing, posting or sending, after `Enter` on a
      conversation over a stale reply, and after a search jump to one.
    - `←`, `→`, `Home`, `End` and `Delete` change nothing (`world`, the
      profile, the overlays, the toast) on Home, in the post view, on My
      Network, Jobs, Messaging (list and with a thread open),
      Notifications and on a member's profile and yours.
  - Job filters:
    - `f` steps All → Remote → Hybrid → On-site → Saved → All with no
      toast, showing exactly the seed's six jobs, then Senior Rust Engineer
      and Developer Advocate, Terminal UI Designer and Engineering Manager,
      Site Reliability Engineer and Data Engineer, and the Senior Rust
      Engineer alone.
    - From Developer Advocate scrolled two lines: Remote keeps him and the
      scroll; Hybrid, On-site and Saved each select their first job from
      the top; All keeps the Senior Rust Engineer and his scroll.
    - Under Hybrid, `k`/`↑` on the first job and `j`/`↓` on the last keep
      the job and the scroll, and `j` goes from Terminal UI Designer
      straight to Engineering Manager. With one job neither moves.
    - The empty Saved list: no selected job, and `s`, `e`, `j`, `k`, `J`,
      `K`, `↓`, `↑`, `Enter` and `v` change nothing (selections, jobs, the
      scroll, the filter, the toast); `f` goes to All on the Senior Rust
      Engineer. Leaving an empty list after unsaving the Data Engineer last
      selects All's first job, not him.
    - Unsaving under Saved, with three saved jobs, from the first row (the
      next job selected, scroll 0), the middle row (the next) and the last
      (the new last), each "Job unsaved", then down to none. The saved count
      follows and All still lists six.
    - `s` under All and Remote keeps the job, the scroll and the list;
      Saved then keeps a job saved under Remote selected.
    - Search jumps: under Hybrid to the Senior Rust Engineer (`lisbon`),
      the filter goes back to All with him selected at the top; under Remote
      to Developer Advocate (`brightkettle`) it stays Remote; from Home under
      Saved, and from an empty Saved list, it goes back to All. `engineer`
      finds the same four jobs under every filter.
    - The filter and the selected job survive `1` then `3`, `Tab` then
      `BackTab`, `/` then Esc, a profile closed with Esc and with `3`, `s`,
      `s`, `e`, `e`, `J`, `K` and a search jump to a post.
    - `f` changes nothing (`world`, the profile, the filter, no toast) on
      Home, in the post view, on a profile from Home and from My Network, on
      My Network, Messaging (list and after closing a thread) and
      Notifications. It is typed in search on all five tabs, the composer,
      the comment box, the Send picker from the feed and the post view, and
      the reply box, and never filters.
- **`ui.rs`** (101), rendering into a `TestBackend`:
  - The helpers `age`, `thousands`, `count`, `wrap`, `hard_wrap`,
    `truncate` and `first_visible`, plus `width_of`, `head` and `tail`
    (`around_cursor` and `wrap_with_cursor` under "Cursor rendering"). The
    text helpers are also checked on double-width text.
  - At 120×30, Home shows all three columns, in order. At 90×30 the sidebar
    is gone, and at 70×30 the profile card is too. The exact thresholds: the
    sidebar shows at 100 but not 99, and the card at 80 but not 79.
  - Every size from 1×1 to 140×40 renders without panicking, for 37 states:
    Home, Home with two reposts around a published post (Maren Holt's
    repost selected), post view, the post view of a repost, comment box (empty and with text), post view after
    commenting, composer (empty and with text), My Network (invitation and
    suggestion selected), Jobs (with and without a toast, scrolled two lines,
    and scrolled past the end so it draws as the bottom at every size), Messaging, reply
    box (with text and empty), Notifications, search (with matches, empty, no match, and over
    the post view), the Send picker (from the feed, and with `o` over the
    post view), and profiles: Maren Holt's from the feed, Tobias
    Ferreira-Lund's from the post view, an invitation's, an accepted
    invitation's, a suggestion's (connect and pending), Ines Okonkwo's from
    Messaging, yours with two posts, and search over a profile, and Jobs
    filtered to Remote, to Saved (one job) and to an empty Saved list. One
    thread per state.
  - Card counts and `Like` → `Liked`. A published post renders first with
    `now`. The post view shows the full body and comments.
  - Reposts at 120×30: after `r` on Maren Holt's post the first card is
    exactly the card in "Reposts" above, row for row, with a dim
    `reposted this` line and a gray border, and her own card below it is
    selected, with the same lines and styles and `Reposted` bold light blue
    on both. At 120×40 Tobias Ferreira-Lund's card, which isn't reposted, has
    the same cells as without the repost, one card lower. After `l` on the
    repost, `343 reactions · 3 comments` and `Liked  Comment  Reposted  Send`
    show twice, and unliking on the original puts both back.
  - The post view of a repost shows `Reposted`, the original's comments and
    no `reposted this` line, and equals the original's post view cell for
    cell. A comment posted there shows `4 comments` in it and twice on the
    feed, and in the original's post view.
  - The `reposted this` line needs 35 columns of terminal below 80. With
    the repost selected it reads exactly `Jordan Vale reposted this · now`
    at 35, `Jorda… reposted this · now` at 30, `J… reposted this · now` at
    26, `Jordan Vale reposted…` at 25, `Jordan Vale…` at 24, 20 and 16,
    `Jordan…` at 12 and `Jor…` at 8. At every width from 6 to 35 the row
    keeps both borders and is exactly the terminal's width, and inside them
    it is the whole line, the name cut with the whole rest after it, or
    whole words ending in `…` (not on the `·`); a cut first word only
    where `Jordan…` doesn't fit.
  - My Network, Jobs, Messaging and Notifications render the same at
    120×30, 80×24 and 60×20 with two reposts and a published post in the
    feed as with the seed feed.
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
    after `fit_job_details` (before it, all but the footer, which still
    reads `J/K scroll` for the screen the app was last fitted to; the event
    loop never draws that frame); at 60×23 the end stays on the last row and
    the first `K` moves the rows down one.
  - The details card at the top, scrolled two lines and past the end, at
    every size from 1×1 to 140×40: the corners are always `╭╮╰╯`, ` ↑ more `
    shows exactly when lines are hidden above and it fits beside the whole
    title, ` ↓ more ` exactly when lines are hidden below and it fits, a
    one-row card shows neither, and at the bottom the last row inside is
    never blank.
  - The Jobs footer, fitted to 80×20 where the Senior Rust Engineer's
    details overflow, reads exactly
    `j/k move  s save  e easy apply  f filter  1-5 tabs  q quit  J/K scroll`.
    At every narrower width it is that line cut on the right, and it starts
    with exactly what the footer without `J/K scroll` showed at that width.
    Fitted to 80×24, where they fit, it has no `J/K scroll`.
    No other tab's footer has `J/K`. Home, My Network, Messaging and
    Notifications render the same at 120×30, 80×20 and 60×20 with the Jobs
    details scrolled as without.
  - My Network, Jobs and Messaging contents, including `◷ Pending`,
    `✔ Applied`, unread markers and the reply box.
  - Every state's hint set is at most 80 columns, and is what the footer
    shows at 80×24. `every_state` holds 27 different sets: the 11 base
    footers (the post view with someone else's comment selected, and each
    text box empty, and Jobs fitted to 60×20, with `J/K scroll`), the post
    view with your comment, 7 profile footers, the 5 text boxes holding
    text, which add `←/→ cursor`, and the Jobs footers with nothing fitted
    (no `J/K scroll`), for one shown job and for an empty list. The post view footer reads exactly
    `j/k comments  l like  c comment  r repost  s send  v profile  esc back  q quit`
    (80 columns with the padding) and the
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
  - Profiles at 120×30: Maren Holt's top card reads `Maren Holt · 1st` with
    `Message` (on blue) on the right, the headline and `Lisbon, Portugal ·
    38 mutual connections`; About's first row; Experience's four lines;
    Activity's first line and `342 reactions · 3 comments` with the bar and
    background on the selected post, Activity's border light blue and the
    top card's gray; the header still shows `1 Home`.
  - The action follows the person: `Ignore   Accept` and `No recent posts`
    for Baptiste Moreau, then `1st` and no button after `a`; `2nd` and no
    button after `x` on Noor Haddad; `+ Connect`, `◷ Pending`, `+ Connect`
    for Amara Lindgren with `c` and `cc`; nothing for Tobias Ferreira-Lund.
  - Your profile: `Jordan Vale` with no degree, your headline,
    `312 connections` (313 after accepting), no About, Experience or
    button, and Activity newest first, the bar following `j`.
  - Live counts: after a like and a comment, Maren Holt's Activity shows
    `343 reactions · 4 comments`.
  - At 80×24 every one of the 16 members' profiles shows all four cards and
    its headline.
  - Short screens: an invitation's profile drops About at 60×20 and
    Experience too at 30×16, and at every height from 4 to 40 the top card
    is first at the top of the body. With four posts by Maren Holt at 80×24,
    all four cards still show and `j` three times brings the fourth into
    Activity's rows. With twelve posts of yours at 80×24 and the last
    selected, it is on screen and so is your name.
  - The top card at 38 columns keeps `Theo Vasquez-Hart · 2nd  + Connect` on
    one row; at 37 `+ Connect` moves to a row of its own; at 20 the name is
    cut and the degree kept: `│ Theo Vasq… · 2nd │`.
  - Every profile state from `every_state`, your profile with a published
    post of wide text, and three My Network profiles, at every size from
    1×1 to 140×40: every card `profile_cards` lays out is inside the body,
    and every card at least 2×2 has every row starting and ending with its
    border and exactly the card's width, by display width. The wide post's
    Activity line is cut with `…` to within one column of its row at 60×20.
  - The footers read exactly as listed under "Layout" for Home, the post
    view, My Network and Messaging, and for profiles of an invitation, a
    suggestion (connect and withdraw), Maren Holt and you, at 80×24. Home
    is 75 columns, the post view and My Network exactly 80. The hints and
    every profile label are single width, and so is every member string in
    the seed check.
  - Profile footers, exhaustively: for an invitation, a suggestion (connect
    and pending), a connection with a conversation, one without, an
    accepted and an ignored invitation, and you, each with 0, 1 and 2 posts
    (`profile_with_posts` gives them seed posts), the footer is exactly
    `[j/k posts  ][enter open  ][action  ]esc back  1-5 tabs  q quit`, with
    `j/k posts` only at 2 and `enter open` from 1. None is wider than 71,
    and an invitation with two posts is exactly 71.
  - The post view footer: on Maren Holt's post, `r repost` with a seed
    comment selected, `x delete` in its place (80 columns) once your comment
    is posted and selected, `r repost` again after `k` and after deleting it.
    On Lucia Brandt's post, no `j/k comments` with no comments or only your
    one, and `x delete` for yours, in the original's post view and in the
    post view of its repost.
  - Cursor rendering:
    - `around_cursor` is exactly `tail` with the cursor at the end and
      `head` from the start with it at the start, at every width, for ASCII
      and wide text; it shows a text that fits whole; it keeps the end while
      a third of the width is before the cursor and otherwise puts the
      cursor a third in; it never splits `日`. For every cursor and width up
      to 30 the two parts meet at the cursor, fit the width, and the
      window's start never moves against the cursor.
    - `wrap_with_cursor` puts the `▏` on the right row, at the start, the
      middle and the end, and next to `日`.
    - At 120×30, `Nice one` with the cursor four steps left shows
      `Nice▏ one` in the comment box, the reply box and the composer,
      `To Nice▏ one` in the picker, `⌕ Nice▏ one` in search and in the top
      bar. In the comment box the cells read exactly `Nice▏ one`, only the
      `▏` light blue.
    - `Start ` and four times the wide text, at 120×30, 80×24 and 60×20, in
      the five boxes: after `Home` the row starts with the `▏`, then
      `Start a日`; after `Home` then `End` the screen is identical to the
      one before `Home`, and in the one-line boxes it shows the end of the
      text. The top bar shows `⌕ ▏Start a日` after `Home`.
    - A composer draft four rows and more taller than the box: with the
      cursor at the start the first rows show, exactly; with it on a middle
      row that row is at the top; at the end, as before, the last row ends
      in the `▏`.
    - The five boxes empty, and with 3,000 characters of `日本語 text` and
      the cursor at the start, in the middle, at the end, just after a `日`
      and just before one, at every size from 1×1 to 140×40: every row of
      the box starts and ends with its border and is exactly the box's
      width, by display width, and wherever the box has at least one row
      and one column inside its border and padding, the `▏` is inside it
      exactly once. With search open, the top bar's search box holds it
      exactly once whenever it shows.
    - Short boxes: `thread_areas` gives the reply box `min(height, 3)` rows
      at the bottom at every height from 0 to 40, the rows above taking the
      rest. On a 9-row screen the overlays have one row inside, and it is
      the input row: `To ▏` at 12 columns, `To f▏e` for the picker and
      `⌕ t▏y` for search at 30. `search_input` at 2 and 1 columns reads `⌕▏`
      and `▏`, `send_input` at 3 reads `To▏`, and empty at 1, `▏`. The
      composer at 40 columns shows `Jordan Vale`, a blank line and `draft▏`
      on a 9-row screen, then drops the blank line at 8 and the name at 7.
    - Each text box's footer at 80×24 reads exactly its old set while empty
      and that set plus `←/→ cursor` with text; the hints are single width;
      `←` changes the screen there, and `→` changes it back.
  - Every key a footer names does something, and the ones a profile footer
    leaves out do nothing (`every_key_a_footer_names_does_something`). For
    15 states (profiles of every kind with 0, 1, 2 or 3 posts, post views
    with and without your comment and with no comments, and Jobs with an
    empty Saved list), each hinted key (for `j/k`, `j` or `k`; for `1-5`,
    one of the digits) changes the 120×30 screen or quits. Each of `j`,
    `k`, `Enter`, `a`, `x`, `c` and `m` whose hint is absent from a profile
    footer changes nothing, neither do `j`, `k` and `x` when the post view
    leaves them out, nor `j`, `k`, `s`, `e`, `J` and `K` on the empty Jobs
    list.
  - Job filters:
    - The filter row at 120×30 reads `All  Remote  Hybrid  On-site  Saved`
      for every filter, and at 80×24 `All  Remote  Hybrid …` (All, Remote,
      Hybrid), `… Remote  Hybrid  On-site …` and `… Hybrid  On-site
      Saved`. At both sizes exactly the active pill's cells are underlined,
      bold and light blue, and every other letter and `…` on the row is dim.
    - Each filter lists exactly its jobs at 120×40, in seed order, three
      rows each from the row under the pills, the first with the `▌` bar.
    - The title at 120×30 and 80×24 is ` Jobs for you ` for All and
      ` Jobs for you · Remote (2) `, `Hybrid (2)`, `On-site (2)`,
      `Saved (1)` and, empty, `Saved (0)`; at 60×20 ` Remote (2) `. At every
      width from 1 to 140 the top border keeps both corners and the title is
      the long one whenever it fits, else the short one whenever that fits,
      else the short one cut with `…`.
    - The empty Saved list shows `No saved jobs` and `Select a job`, no bar,
      no job, no details and no indicators at 120×30, 80×24 and 60×20, with
      `job_scroll_max` 0; with no Remote jobs left, Remote shows `No jobs
      match` and `Remote (0)`. Unsaving the only saved job under Saved
      shows the toast, the empty list and `Saved (0)`, and Home's profile
      card `Saved jobs 0`.
    - `filter_row` for every filter at every width from 0 to 60: never
      wider than the width; whole pills in seed order holding the active
      label whenever it fits, with a `…` exactly on each side that leaves
      pills out; the label cut with `…` only when it doesn't fit; the full
      row from 36.
    - The list card for every filter, the empty Saved list and an empty
      Remote list at every size from 1×1 to 140×40: every row starts and
      ends with its border, and the active label is on the filter row
      whenever the card is tall enough and its inside as wide as the label.
    - Footers fitted as the event loop fits them: at 80×24 (everything
      fits) `j/k move  s save  e easy apply  f filter  1-5 tabs  q quit`
      for All, `s save  e easy apply  f filter  1-5 tabs  q quit` for the
      one saved job (where `j` and `k` change nothing) and
      `f filter  1-5 tabs  q quit` for none; at 80×20 the same plus
      `J/K scroll` for All (72 columns), Remote and the one saved job, and
      still nothing for none. In each, `J/K scroll` is there exactly when
      `J` changes the screen. No other tab says `filter`.
    - Under every filter at 60×20, `J` fitted before each key stops where
      ` ↓ more ` goes and ` ↑ more ` shows, with the last row inside the
      card not blank, and the first `K` brings ` ↓ more ` back.

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

For reposting, each break was caught and then restored: leaving `post_sel`
alone on `r` (so the selection slides onto the repost) fails 10, including
`r_reposts_to_the_top_and_keeps_the_selection_in_the_feed`,
`r_in_the_post_view_keeps_the_post_and_comment`,
`a_repost_card_shows_the_original_under_one_line` and every test built on
`every_state`; making `App::reposted` always false fails 6 (including
`a_post_is_reposted_once`, `liking_either_card_changes_both` and
`the_post_view_of_a_repost_is_the_originals`); jumping to `post_sel = i`
instead of the post's own card fails
`search_finds_a_reposted_post_once_and_lands_on_the_original`; acting on
`posts[post_sel]` instead of the card's post fails
`likes_and_comments_from_a_repost_change_the_original` and
`liking_either_card_changes_both`; treating a `PostId` as an index into
`posts` fails 6, including
`reposts_keep_their_original_when_cards_go_in_above` and
`your_own_post_can_be_reposted`; and cutting the whole `reposted this` line
instead of the name fails `a_narrow_reposted_line_is_cut_inside_the_card`.
So does going back to the first version of `reposted_line`, which kept a
lone `…` and let the border clip the rest (`… reposted t`). Giving every
published post `PostId(0)` fails 6, including `post_ids_are_unique` and
`reposts_keep_their_original_when_cards_go_in_above`; publishing a post
without its card fails 13, including `post_ids_are_unique` (through
`assert_feed_matches_posts`) and every test built on `every_state`.

For profile pages, each break was caught and then restored: making `Esc`
on a profile also drop to Browse fails
`esc_returns_to_exactly_where_the_profile_was_opened`; not setting
`connected` on accept fails `accepting_on_a_profile_makes_them_first_degree`
and `the_action_follows_the_person`; listing Activity oldest first fails 3
(`your_own_profile_lists_your_posts_newest_first`,
`your_own_profile_shows_your_connections_and_posts` and
`short_screens_drop_about_then_experience_but_keep_the_selection`); landing
`go_to_post` on `post_sel = i` instead of the post's own card fails
`enter_on_activity_opens_the_post_on_its_own_card` and
`search_finds_a_reposted_post_once_and_lands_on_the_original`; letting
`set_tab` keep the profile open fails 4, including
`switching_tabs_closes_the_profile` and
`m_opens_the_conversation_like_messaging_does`; dropping the clamp from
`act_on_network_item` fails `removal_keeps_selection_in_bounds` and
`removing_the_last_row_from_a_profile_clamps_the_selection`; leaving
`open_conversation` out of `go_to_conversation` fails 3, including
`m_opens_the_conversation_like_messaging_does`; making `v` in the post view
open the selected comment's author fails 4, including
`v_opens_the_profile_under_the_selection_everywhere`; cutting the Activity
line by `char` count fails `profiles_stay_inside_their_borders_at_every_size`;
always putting the action beside the name fails
`a_narrow_top_card_puts_the_action_on_its_own_row`; and letting Activity
claim its whole height instead of `ACTIVITY_MIN_HEIGHT` fails
`short_screens_drop_about_then_experience_but_keep_the_selection`. That last
break first passed every test, since no seed member has more than one post;
the test now gives Maren Holt four posts to catch it.

For the footers that follow the state, each break was caught and then
restored. Showing `j/k posts` with one post, or `enter open` with none,
fails 4 each (`profile_footers_name_only_what_works`,
`profile_hints_and_labels`, `every_key_a_footer_names_does_something` and
`every_hint_set_fits_80_columns`). Never showing `x delete`, or showing it
beside `r repost` instead of in its place (90 columns), fails
`the_post_view_footer_names_x_delete_only_for_your_comment` and
`every_hint_set_fits_80_columns`. Always showing `j/k comments` fails
`the_post_view_footer_names_x_delete_only_for_your_comment` and
`every_key_a_footer_names_does_something`. Making `is_yours` true for any
post holding a comment of yours fails
`the_post_view_footer_names_x_delete_only_for_your_comment`.

For the movable cursor, each break was caught and then restored. Selecting
the first result on every key, moves included, fails
`only_a_real_change_to_the_query_selects_the_first_result_again`. Always
showing the end of a one-line box (`tail`) whatever the cursor fails
`around_cursor_shows_the_end_the_start_or_the_cursor_a_third_in` and
`a_long_text_shows_its_start_with_the_cursor_there_and_its_end_as_before`.
Always showing the end of the composer's draft fails
`a_composer_draft_taller_than_the_box_shows_the_cursors_row` and
`every_box_keeps_its_cursor_inside_its_border_at_every_size`. Not putting
the cursor back to 0 in `TextField::clear` fails 18, including
`every_box_opens_empty_with_the_cursor_at_0` (and panics elsewhere once the
next character goes in past the end). Showing `←/→ cursor` in an empty box
fails 4, including
`text_box_footers_name_the_cursor_keys_once_there_is_text` and
`every_hint_set_fits_80_columns`. Drawing the `▏` over the next character
fails `the_cursor_shows_in_the_middle_of_every_box` and the long-text test.
Making `Backspace` delete the last character instead of the one before
the cursor fails 4, and stepping `←` one byte instead of one `char` fails
`a_double_width_character_is_one_step` and
`every_text_box_moves_the_cursor_and_edits_in_the_middle`. Letting
Ctrl/Alt-modified keys through except Ctrl with a letter fails 5,
including the three "types keys instead of acting on them" tests.

For the cursor always showing, each fix was undone and caught, then
restored. Going back to the Send picker's single four-row `Layout` (which
gave a lone inside row to the post line, first at 9×9), keeping the
whole `⌕ ` in a row under 3 columns (first at 9×9), and always keeping the
composer's name and blank line (first at 9×7) each fail
`every_box_keeps_its_cursor_inside_its_border_at_every_size` and
`short_boxes_keep_their_input_row`. Going back to `thread_areas`' single
four-row `Layout`, which squeezes the reply box to 1 or 2 rows on a thread
with 3 or 4 rows inside, fails `short_boxes_keep_their_input_row` (the
every-size sweep can't see this one, since a reply box squeezed below 3
rows has no row inside).

For the job filters, each break was caught and then restored. Making `f`
always select the filter's first job fails
`the_filter_keeps_the_selected_job_or_moves_to_the_first_match` and
`unsaving_under_saved_moves_to_the_next_job`; not resetting the scroll when
`f` changes the job fails the first of those; leaving the selection alone
when `s` drops a job from Saved fails
`unsaving_under_saved_moves_to_the_next_job` and
`leaving_an_empty_list_selects_the_first_match`; keeping a filter that hides
the jumped-to job, or always going back to All, fails
`a_search_jump_shows_the_job_even_when_the_filter_hides_it`; always drawing
the whole filter row fails 3 (`the_filter_row_marks_the_active_pill_at_120_and_80`,
`the_filter_row_keeps_the_active_label_and_fits_at_every_width` and
`the_jobs_list_stays_inside_its_card_at_every_size`); always showing the
full Jobs footer fails 3 (`jobs_footers_name_only_what_works`,
`every_hint_set_fits_80_columns` and
`every_key_a_footer_names_does_something`); showing `J/K scroll` whatever
`job_scroll_max` fails 3 (`jobs_footers_name_only_what_works`,
`every_hint_set_fits_80_columns` and `the_jobs_footer_shows_j_k_scroll`);
always using the long title
fails `the_list_title_counts_the_jobs_shown`; and letting `selected_job`
ignore the filter fails 6, including
`an_empty_saved_list_ignores_every_job_key` and
`an_empty_list_says_so_and_selects_no_job`.

After the job filters, the binary was run in a 120×30 private tmux session
(`tmux -L wfilter`): `3` and `f` gave the Jobs capture above; `f` three
more times and `s` showed "Job unsaved", `Jobs for you · Saved (0)` and
`No saved jobs`; resized to 80×24, `f` then three more `f` showed
`Jobs for you · On-site (2)` over `… Remote  Hybrid  On-site …`. `q` quit.
After `J/K scroll` learned to follow the screen, in an 80×24 session
(`tmux -L wfilter2`): `3` showed the footer without it, `f` four times
`s save  e easy apply  f filter  1-5 tabs  q quit`, `s` (and Esc for the
toast) `f filter  1-5 tabs  q quit`, and `f` then a resize to 80×20 brought
`J/K scroll` back. The capture above was taken again then.

After the movable cursor, the binary was run in a 120×30 private tmux
session (`tmux -L wcursor`): the comment, Send picker and search captures
above came out as before but for the footer, and `←` and `Home` moved the
`▏` as captured.

## Ideas

Mouse support.
