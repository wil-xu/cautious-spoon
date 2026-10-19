use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Clear;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::app::Mode;
use crate::app::NetworkItem;
use crate::app::Search;
use crate::app::SearchHit;
use crate::app::Tab;
use crate::app::search_needle;
use crate::data::Post;

const BLUE: Color = Color::Rgb(10, 102, 194);
const LIGHT_BLUE: Color = Color::Rgb(112, 181, 249);
const RED: Color = Color::Rgb(204, 16, 22);
const GREEN: Color = Color::Rgb(87, 166, 74);
const DIM: Color = Color::Rgb(140, 140, 140);
const BORDER: Color = Color::Rgb(66, 66, 66);
const HIGHLIGHT_BG: Color = Color::Rgb(29, 40, 54);

const PROFILE_WIDTH: u16 = 24;
const SIDEBAR_WIDTH: u16 = 30;
/// The Home sidebar shows at this terminal width or more.
const SIDEBAR_MIN_WIDTH: u16 = 100;
/// The Home profile card shows at this terminal width or more.
const PROFILE_MIN_WIDTH: u16 = 80;
const CARD_BODY_LINES: usize = 3;
const MESSAGING_LIST_WIDTH: u16 = 34;

/// `now`, `3h`, `2d` or `1w`, as LinkedIn shows on a card.
fn age(hours: u32) -> String {
    match hours {
        0 => "now".to_owned(),
        1..24 => format!("{hours}h"),
        24..168 => format!("{}d", hours / 24),
        _ => format!("{}w", hours / 168),
    }
}

/// `1,204`.
fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// `1 reaction`, `1,204 reactions`.
fn count(n: u32, noun: &str) -> String {
    let s = if n == 1 { "" } else { "s" };
    format!("{} {noun}{s}", thousands(n))
}

/// Display width in terminal columns. Typed text can hold characters such as
/// `日` that take two columns, so every measurement here uses this rather than
/// the `char` count.
fn width_of(text: &str) -> usize {
    Span::raw(text).width()
}

/// The longest start of `text` that fits in `width` columns.
fn head(text: &str, width: usize) -> &str {
    let mut end = 0;
    for (i, ch) in text.char_indices() {
        let next = i + ch.len_utf8();
        if width_of(&text[..next]) > width {
            break;
        }
        end = next;
    }
    &text[..end]
}

/// The longest end of `text` that fits in `width` columns.
fn tail(text: &str, width: usize) -> &str {
    let mut start = text.len();
    for (i, _) in text.char_indices().rev() {
        if width_of(&text[i..]) > width {
            break;
        }
        start = i;
    }
    &text[start..]
}

/// Splits typed text into rows of at most `width` columns, keeping every
/// space, so a text box shows exactly what was typed. A character wider than
/// `width` gets a row of its own.
fn hard_wrap(text: &str, width: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut rest = text;
    loop {
        let mut row = head(rest, width);
        if row.is_empty() {
            // Always make progress, even when one character doesn't fit.
            row = rest.char_indices().nth(1).map_or(rest, |(i, _)| &rest[..i]);
        }
        rows.push(row.to_owned());
        rest = &rest[row.len()..];
        if rest.is_empty() {
            return rows;
        }
    }
}

/// Greedy word wrap by display width. Words wider than `width` are
/// hard-split, and newlines start a new line.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    text.split('\n')
        .flat_map(|para| {
            let mut lines = Vec::new();
            let mut cur = String::new();
            for word in para.split_whitespace() {
                let mut pieces = hard_wrap(word, width);
                let word = pieces.pop().expect("hard_wrap returns at least one row");
                for piece in pieces {
                    if !cur.is_empty() {
                        lines.push(std::mem::take(&mut cur));
                    }
                    lines.push(piece);
                }
                let needed = width_of(&cur) + usize::from(!cur.is_empty()) + width_of(&word);
                if needed > width && !cur.is_empty() {
                    lines.push(std::mem::take(&mut cur));
                }
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(&word);
            }
            lines.push(cur);
            lines
        })
        .collect()
}

/// `text` cut to `width` columns, ending in `…` when cut.
fn truncate(text: &str, width: usize) -> String {
    if width_of(text) <= width {
        return text.to_owned();
    }
    format!("{}…", head(text, width.saturating_sub(1)))
}

/// `left` and `right` on one line of `width` columns, with the gap between.
fn spread(left: Vec<Span<'static>>, right: Vec<Span<'static>>, width: usize) -> Line<'static> {
    let used: usize = left.iter().chain(&right).map(Span::width).sum();
    let gap = Span::raw(" ".repeat(width.saturating_sub(used).max(1)));
    Line::from(
        left.into_iter()
            .chain([gap])
            .chain(right)
            .collect::<Vec<_>>(),
    )
}

fn stat_line(label: &'static str, value: usize, width: usize) -> Line<'static> {
    spread(
        vec![Span::styled(label, Style::new().fg(DIM))],
        vec![Span::styled(
            value.to_string(),
            Style::new().fg(LIGHT_BLUE).bold(),
        )],
        width,
    )
}

fn card(title: &str, selected: bool) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(if selected { LIGHT_BLUE } else { BORDER })
        .title(Span::styled(title, Style::new().bold()))
        .padding(Padding::horizontal(1))
}

fn usize_of(n: u16) -> usize {
    usize::from(n)
}

fn u16_of(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The first item to draw so that the selected item fits in `rows`, starting
/// from the top: the list only scrolls once the selection would fall off the
/// bottom.
fn first_visible(heights: &[usize], selected: usize, rows: usize) -> usize {
    let Some(last) = heights.len().checked_sub(1) else {
        return 0;
    };
    let selected = selected.min(last);
    let mut first = 0;
    while first < selected && heights[first..=selected].iter().sum::<usize>() > rows {
        first += 1;
    }
    first
}

/// Draws items of one or more lines each, top to bottom. The selected item
/// gets a blue bar and a highlighted background.
fn render_list(
    frame: &mut Frame,
    area: Rect,
    items: Vec<Vec<Line<'static>>>,
    selected: Option<usize>,
) {
    let heights: Vec<usize> = items.iter().map(Vec::len).collect();
    let first = selected.map_or(0, |s| first_visible(&heights, s, usize_of(area.height)));
    let mut y = area.y;
    for (i, lines) in items.into_iter().enumerate().skip(first) {
        if y >= area.bottom() {
            break;
        }
        let h = u16_of(lines.len()).min(area.bottom() - y);
        let is_selected = selected == Some(i);
        let bar = Span::styled(if is_selected { "▌" } else { " " }, Style::new().fg(BLUE));
        let lines: Vec<Line> = lines
            .into_iter()
            .map(|line| {
                let mut spans = vec![bar.clone()];
                spans.extend(line.spans);
                Line::from(spans)
            })
            .collect();
        let bg = if is_selected {
            HIGHLIGHT_BG
        } else {
            Color::Reset
        };
        frame.render_widget(
            Paragraph::new(lines).bg(bg),
            Rect::new(area.x, y, area.width, h),
        );
        y += h;
    }
}

/// The header, body and footer of a screen of size `area`.
fn screen_areas(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area)
}

pub(crate) fn draw(frame: &mut Frame, app: &App) {
    let [header, body, footer] = screen_areas(frame.area());

    render_header(frame, app, header);
    render_footer(frame, app, footer);
    match app.tab {
        Tab::Home => render_home(frame, app, body),
        Tab::Network => render_network(frame, app, body),
        Tab::Jobs => render_jobs(frame, app, body),
        Tab::Messaging => render_messaging(frame, app, body),
        Tab::Notifications => render_notifications(frame, app, body),
    }
    if app.mode == Mode::Composer {
        render_composer(frame, app, body);
    }
    if let Some(search) = &app.search {
        render_search(frame, app, search, body);
    }
}

/// How much of each tab label fits in the header. The header tries them in
/// this order, after first dropping the search box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabStyle {
    /// `2 My Network (3)`.
    Full,
    /// `2 Network (3)`.
    Short,
    /// `2 (3)`, except the active tab, which keeps its short label.
    Numbers,
    /// `2 (3)` for every tab, one space apart.
    Bare,
}

impl TabStyle {
    const ALL: [TabStyle; 4] = [
        TabStyle::Full,
        TabStyle::Short,
        TabStyle::Numbers,
        TabStyle::Bare,
    ];
}

fn short_label(tab: Tab) -> &'static str {
    match tab {
        Tab::Home => "Home",
        Tab::Network => "Network",
        Tab::Jobs => "Jobs",
        Tab::Messaging => "Msgs",
        Tab::Notifications => "Notifs",
    }
}

/// `4 Messaging (2)`, or `4 Messaging` when there's nothing unread, with the
/// label shortened or dropped for narrow headers.
fn tab_label(app: &App, tab: Tab, style: TabStyle) -> (String, Option<String>) {
    let n = Tab::ALL
        .iter()
        .position(|&t| t == tab)
        .expect("every tab is listed in Tab::ALL")
        + 1;
    let badge = match app.badge(tab) {
        0 => None,
        b => Some(format!(" ({b})")),
    };
    let label = match style {
        TabStyle::Full => format!("{n} {}", tab.label()),
        TabStyle::Short => format!("{n} {}", short_label(tab)),
        TabStyle::Numbers if tab == app.tab => format!("{n} {}", short_label(tab)),
        TabStyle::Numbers | TabStyle::Bare => n.to_string(),
    };
    (label, badge)
}

fn tabs_line(app: &App, style: TabStyle) -> Line<'static> {
    let gap = if style == TabStyle::Bare { " " } else { "  " };
    let mut spans = Vec::new();
    for (i, &tab) in Tab::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(gap));
        }
        let (label, badge) = tab_label(app, tab, style);
        let style = if tab == app.tab {
            Style::new()
                .fg(Color::White)
                .bold()
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::new().fg(DIM)
        };
        spans.push(Span::styled(label, style));
        if let Some(badge) = badge {
            spans.push(Span::styled(badge, Style::new().fg(RED).bold()));
        }
    }
    Line::from(spans)
}

const LOGO_WIDTH: u16 = 4;
const SEARCH_MIN_WIDTH: u16 = 10;
const HEADER_GAP: u16 = 2;

/// Whether the search box shows, and the tab labels, for a header whose
/// inside is `width` columns: the search box goes first, then the labels
/// shorten. The logo always stays.
fn header_fit(app: &App, width: u16) -> (bool, Line<'static>) {
    for style in TabStyle::ALL {
        let tabs = tabs_line(app, style);
        let needed = LOGO_WIDTH + HEADER_GAP + u16_of(tabs.width());
        if style == TabStyle::Full && needed + HEADER_GAP + SEARCH_MIN_WIDTH <= width {
            return (true, tabs);
        }
        if needed <= width {
            return (false, tabs);
        }
    }
    (false, tabs_line(app, TabStyle::Bare))
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(BORDER);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (show_search, tabs) = header_fit(app, inner.width);
    let [logo, rest] = Layout::horizontal([Constraint::Length(LOGO_WIDTH), Constraint::Fill(1)])
        .spacing(HEADER_GAP)
        .areas(inner);
    frame.render_widget(
        Span::styled(" in ", Style::new().fg(Color::White).bg(BLUE).bold()),
        logo,
    );

    let tabs_width = u16_of(tabs.width());
    if show_search {
        let [search, tabs_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(tabs_width)])
                .spacing(HEADER_GAP)
                .areas(rest);
        let text = match &app.search {
            // Two columns of padding and the icon, then the cursor.
            Some(s) => vec![
                Span::styled(" ⌕ ", Style::new().fg(LIGHT_BLUE)),
                Span::styled(
                    tail(&s.query, usize_of(search.width).saturating_sub(4)).to_owned(),
                    Style::new().fg(Color::White),
                ),
                Span::styled("▏", Style::new().fg(LIGHT_BLUE)),
            ],
            None => vec![
                Span::styled(" ⌕ ", Style::new().fg(DIM)),
                Span::styled("Search", Style::new().fg(DIM)),
            ],
        };
        frame.render_widget(Paragraph::new(Line::from(text)).bg(HIGHLIGHT_BG), search);
        frame.render_widget(tabs, tabs_area);
    } else if tabs_width <= rest.width {
        frame.render_widget(tabs.right_aligned(), rest);
    } else {
        // Too narrow for even the bare tabs: keep the first ones visible.
        frame.render_widget(tabs, rest);
    }
}

/// Key hints for the current tab and mode. Each set fits on one line at 80
/// columns.
fn hints(app: &App) -> &'static [(&'static str, &'static str)] {
    if app.search.is_some() {
        return &[
            ("type", "search"),
            ("↑/↓", "move"),
            ("enter", "open"),
            ("esc", "close"),
        ];
    }
    match (app.tab, app.mode) {
        (_, Mode::Composer) => &[
            ("type", "write"),
            ("backspace", "delete"),
            ("enter", "publish"),
            ("esc", "discard"),
        ],
        (_, Mode::Reply) => &[
            ("type", "write"),
            ("backspace", "delete"),
            ("enter", "send"),
            ("esc", "back to list"),
        ],
        (_, Mode::CommentBox { .. }) => &[
            ("type", "write"),
            ("backspace", "delete"),
            ("enter", "post"),
            ("esc", "cancel"),
        ],
        (_, Mode::PostDetail { .. }) => &[
            ("j/k", "comments"),
            ("l", "like"),
            ("c", "comment"),
            ("x", "delete"),
            ("esc", "back"),
            ("1-5", "tabs"),
            ("q", "quit"),
        ],
        (Tab::Home, Mode::Browse) => &[
            ("j/k", "move"),
            ("l", "like"),
            ("enter", "open"),
            ("p", "post"),
            ("tab", "next"),
            ("1-5", "tabs"),
            ("q", "quit"),
        ],
        (Tab::Network, Mode::Browse) => &[
            ("j/k", "move"),
            ("a", "accept"),
            ("x", "ignore"),
            ("c", "connect"),
            ("tab", "next"),
            ("1-5", "tabs"),
            ("q", "quit"),
        ],
        (Tab::Jobs, Mode::Browse) => &[
            ("j/k", "move"),
            ("s", "save"),
            ("e", "easy apply"),
            ("tab", "next"),
            ("1-5", "tabs"),
            ("q", "quit"),
        ],
        (Tab::Messaging, Mode::Browse) => &[
            ("j/k", "move"),
            ("enter", "open"),
            ("tab", "next"),
            ("1-5", "tabs"),
            ("q", "quit"),
        ],
        (Tab::Notifications, Mode::Browse) => &[
            ("j/k", "move"),
            ("tab", "next"),
            ("1-5", "tabs"),
            ("q", "quit"),
        ],
    }
}

fn hints_line(hints: &[(&'static str, &'static str)]) -> Line<'static> {
    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|&(key, what)| {
            [
                Span::styled(format!(" {key} "), Style::new().fg(LIGHT_BLUE).bold()),
                Span::styled(format!("{what} "), Style::new().fg(DIM)),
            ]
        })
        .collect();
    Line::from(spans)
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let line = match app.status {
        Some(status) => Line::from(Span::styled(
            format!(" {status} "),
            Style::new().fg(Color::White).bg(BLUE).bold(),
        )),
        None => hints_line(hints(app)),
    };
    frame.render_widget(line, area);
}

// ---------------------------------------------------------------- Home

/// Home's profile card, center column and sidebar. A column that doesn't fit
/// is zero wide.
fn home_columns(area: Rect) -> [Rect; 3] {
    let show_profile = area.width >= PROFILE_MIN_WIDTH;
    let show_sidebar = area.width >= SIDEBAR_MIN_WIDTH;
    Layout::horizontal([
        Constraint::Length(if show_profile { PROFILE_WIDTH } else { 0 }),
        Constraint::Min(0),
        Constraint::Length(if show_sidebar { SIDEBAR_WIDTH } else { 0 }),
    ])
    .areas(area)
}

fn render_home(frame: &mut Frame, app: &App, area: Rect) {
    let [left, center, right] = home_columns(area);
    if left.width > 0 {
        render_profile(frame, app, left);
    }
    if right.width > 0 {
        render_news(frame, app, right);
    }
    match app.mode {
        Mode::PostDetail { comment } | Mode::CommentBox { comment } => {
            render_post_detail(frame, app, center, comment);
        }
        Mode::Browse | Mode::Composer | Mode::Reply => render_feed(frame, app, center),
    }
}

fn render_profile(frame: &mut Frame, app: &App, area: Rect) {
    let block = card(" You ", false);
    let width = usize_of(block.inner(area).width);
    let mut lines = vec![Line::styled(
        app.profile.name,
        Style::new().fg(Color::White).bold(),
    )];
    lines.extend(
        wrap(app.profile.headline, width)
            .into_iter()
            .map(|l| Line::styled(l, Style::new().fg(DIM))),
    );
    lines.push(Line::raw(""));
    lines.push(stat_line(
        "Connections",
        usize::try_from(app.profile.connections).unwrap_or(usize::MAX),
        width,
    ));
    lines.push(stat_line("Saved jobs", app.saved_jobs(), width));

    let height = u16_of(lines.len() + 2).min(area.height);
    frame.render_widget(Paragraph::new(lines).block(block), Rect { height, ..area });
}

fn render_news(frame: &mut Frame, app: &App, area: Rect) {
    let block = card(" WorkedIn News ", false);
    let width = usize_of(block.inner(area).width);
    let mut lines = vec![Line::styled("Top stories", Style::new().fg(DIM).bold())];
    for item in &app.news {
        for (i, l) in wrap(item.title, width.saturating_sub(2))
            .into_iter()
            .enumerate()
        {
            let bullet = if i == 0 { "· " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(bullet, Style::new().fg(LIGHT_BLUE)),
                Span::styled(l, Style::new().fg(Color::White).bold()),
            ]));
        }
        lines.push(Line::styled(
            format!(
                "  {} ago · {} readers",
                age(item.hours_ago),
                thousands(item.readers)
            ),
            Style::new().fg(DIM),
        ));
    }
    let height = u16_of(lines.len() + 2).min(area.height);
    frame.render_widget(Paragraph::new(lines).block(block), Rect { height, ..area });
}

fn author_line(author: &'static str, hours_ago: u32, width: usize) -> Line<'static> {
    let when = format!(" · {}", age(hours_ago));
    let name = truncate(author, width.saturating_sub(width_of(&when)));
    Line::from(vec![
        Span::styled(name, Style::new().fg(Color::White).bold()),
        Span::styled(when, Style::new().fg(DIM)),
    ])
}

fn counts_line(post: &Post) -> Line<'static> {
    let comments = u32::try_from(post.comments.len()).unwrap_or(u32::MAX);
    Line::styled(
        format!(
            "{} · {}",
            count(post.reactions, "reaction"),
            count(comments, "comment")
        ),
        Style::new().fg(DIM),
    )
}

fn actions_line(post: &Post) -> Line<'static> {
    let rest = Style::new().fg(Color::Gray);
    let like = if post.liked {
        Span::styled("Liked", Style::new().fg(LIGHT_BLUE).bold())
    } else {
        Span::styled("Like", rest)
    };
    Line::from(vec![like, Span::styled("  Comment  Repost  Send", rest)])
}

/// The body cut to `CARD_BODY_LINES` rows, ending in `…more` when cut.
fn preview(body: &str, width: usize) -> Vec<String> {
    let mut lines = wrap(body, width);
    if lines.len() > CARD_BODY_LINES {
        lines.truncate(CARD_BODY_LINES);
        let last = lines.last_mut().expect("CARD_BODY_LINES is not 0");
        *last = format!("{} …more", head(last, width.saturating_sub(6)).trim_end());
    }
    lines
}

fn card_lines(post: &Post, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        author_line(post.author, post.hours_ago, width),
        Line::styled(truncate(post.headline, width), Style::new().fg(DIM)),
    ];
    lines.extend(
        preview(&post.body, width)
            .into_iter()
            .map(|l| Line::styled(l, Style::new().fg(Color::White))),
    );
    lines.push(counts_line(post));
    lines.push(actions_line(post));
    lines
}

fn render_feed(frame: &mut Frame, app: &App, area: Rect) {
    let [start, feed] = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("✎ ", Style::new().fg(LIGHT_BLUE)),
            Span::styled("Start a post", Style::new().fg(Color::Gray)),
            Span::styled("  (press p)", Style::new().fg(DIM)),
        ]))
        .block(card("", false)),
        start,
    );

    // 2 border columns and 2 padding columns.
    let width = usize_of(feed.width.saturating_sub(4));
    let cards: Vec<Vec<Line>> = app.posts.iter().map(|p| card_lines(p, width)).collect();
    let heights: Vec<usize> = cards.iter().map(|c| c.len() + 2).collect();
    let first = first_visible(&heights, app.post_sel, usize_of(feed.height));

    let mut y = feed.y;
    for (i, lines) in cards.into_iter().enumerate().skip(first) {
        if y >= feed.bottom() {
            break;
        }
        let h = u16_of(lines.len() + 2).min(feed.bottom() - y);
        frame.render_widget(
            Paragraph::new(lines).block(card("", i == app.post_sel)),
            Rect::new(feed.x, y, feed.width, h),
        );
        y += h;
    }
}

const COMMENT_BOX_HEIGHT: u16 = 3;
/// A border and a padding column on each side, and one column of text.
const COMMENT_BOX_MIN_WIDTH: u16 = 5;

/// The Post card's inside, split into the scrolling post and comments, and
/// the comment box under them. The box goes only when it fits whole: then
/// the comments get whatever rows are left, possibly none.
fn post_detail_areas(inner: Rect) -> (Rect, Option<Rect>) {
    if inner.height < COMMENT_BOX_HEIGHT || inner.width < COMMENT_BOX_MIN_WIDTH {
        return (inner, None);
    }
    let [content, comment_box] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(COMMENT_BOX_HEIGHT)]).areas(inner);
    (content, Some(comment_box))
}

fn render_post_detail(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let Some(post) = app.selected_post() else {
        return;
    };
    let block = card(" Post ", true);
    let inner = block.inner(area);
    let width = usize_of(inner.width);
    frame.render_widget(block, area);
    let (content, comment_box) = post_detail_areas(inner);

    let mut lines = vec![author_line(post.author, post.hours_ago, width)];
    lines.extend(
        wrap(post.headline, width)
            .into_iter()
            .map(|l| Line::styled(l, Style::new().fg(DIM))),
    );
    lines.push(Line::raw(""));
    lines.extend(
        wrap(&post.body, width)
            .into_iter()
            .map(|l| Line::styled(l, Style::new().fg(Color::White))),
    );
    lines.push(Line::raw(""));
    lines.push(counts_line(post));
    lines.push(actions_line(post));
    lines.push(Line::styled("─".repeat(width), Style::new().fg(BORDER)));
    if post.comments.is_empty() {
        lines.push(Line::styled("No comments yet", Style::new().fg(DIM)));
    } else {
        lines.push(Line::styled("Comments", Style::new().bold()));
    }

    // Scroll just far enough to show the end of the selected comment.
    let mut selected_end = 0;
    for (i, comment) in post.comments.iter().enumerate() {
        let bar = Span::styled(
            if i == selected { "┃ " } else { "  " },
            Style::new().fg(BLUE),
        );
        let text_width = width.saturating_sub(2);
        let mut block = vec![
            author_line(comment.author, comment.hours_ago, text_width),
            Line::styled(truncate(comment.headline, text_width), Style::new().fg(DIM)),
        ];
        block.extend(
            wrap(&comment.body, text_width)
                .into_iter()
                .map(|l| Line::styled(l, Style::new().fg(Color::Gray))),
        );
        lines.push(Line::raw(""));
        lines.extend(block.into_iter().map(|line| {
            let mut spans = vec![bar.clone()];
            spans.extend(line.spans);
            Line::from(spans)
        }));
        if i == selected {
            selected_end = lines.len();
        }
    }
    let scroll = selected_end.saturating_sub(usize_of(content.height));
    frame.render_widget(Paragraph::new(lines).scroll((u16_of(scroll), 0)), content);

    if let Some(area) = comment_box {
        render_comment_box(frame, app, area);
    }
}

/// The "Add a comment" box at the bottom of the post view, drawn like the
/// Messaging reply box.
fn render_comment_box(frame: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.mode, Mode::CommentBox { .. });
    let block = card(" Add a comment ", focused);
    let box_width = usize_of(block.inner(area).width);
    let cursor = Span::styled("▏", Style::new().fg(LIGHT_BLUE));
    let line = match (focused, app.comment_draft.is_empty()) {
        (false, _) => Line::styled("Press c to comment", Style::new().fg(DIM)),
        (true, true) => Line::from(vec![
            Span::styled(
                head("Add a comment…", box_width.saturating_sub(1)),
                Style::new().fg(DIM),
            ),
            cursor,
        ]),
        (true, false) => {
            // Show the end of a long comment, where the cursor is. One column
            // goes to the cursor.
            let shown = tail(&app.comment_draft, box_width.saturating_sub(1)).to_owned();
            Line::from(vec![Span::raw(shown), cursor])
        }
    };
    frame.render_widget(Paragraph::new(line).block(block), area);
}

/// A rectangle of at most `width`×`height`, centered in `area`.
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// Where the composer box goes over the body.
fn composer_area(body: Rect) -> Rect {
    centered(body, body.width.saturating_sub(4).min(64), 12)
}

fn render_composer(frame: &mut Frame, app: &App, area: Rect) {
    let popup = composer_area(area);
    frame.render_widget(Clear, popup);
    let block = card(" Create a post ", true);
    let inner = block.inner(popup);
    let width = usize_of(inner.width);
    frame.render_widget(block, popup);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(app.profile.name, Style::new().fg(Color::White).bold()),
            Span::styled(" · Post to Anyone", Style::new().fg(DIM)),
        ]),
        Line::raw(""),
    ];
    let cursor = Span::styled("▏", Style::new().fg(LIGHT_BLUE));
    if app.draft.is_empty() {
        lines.push(Line::from(vec![
            cursor,
            Span::styled("What do you want to talk about?", Style::new().fg(DIM)),
        ]));
    } else {
        // Keep the end of a long draft, where the cursor is, in view.
        let rows = usize_of(inner.height).saturating_sub(lines.len()).max(1);
        let text = hard_wrap(&format!("{}▏", app.draft), width);
        let skip = text.len().saturating_sub(rows);
        lines.extend(text.into_iter().skip(skip).map(Line::raw));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

// -------------------------------------------------------------- Search

const SEARCH_PROMPT: &str = "Type to search people, jobs and posts";
const NO_RESULTS: &str = "No results";

/// Where the search overlay goes over the body.
fn search_area(body: Rect) -> Rect {
    centered(
        body,
        body.width.saturating_sub(4).min(80),
        body.height.saturating_sub(2).min(24),
    )
}

/// The byte ranges of `text` that match `needle`, a lowercased query, in
/// order. The match runs on `text.to_lowercase()`, as `App::search_hits`
/// does, but lowercasing can change a string's length (`İ` becomes two
/// characters), so each range is mapped back to whole characters of `text`.
fn match_ranges(text: &str, needle: &str) -> Vec<Range<usize>> {
    if needle.is_empty() {
        return Vec::new();
    }
    // `(offset in lower, offset in text)` at every character of `text`, and
    // at the end. `str::to_lowercase` lowercases each character on its own,
    // except for a final `Σ`, whose two forms are the same length.
    let lower = text.to_lowercase();
    let mut bounds = Vec::new();
    let mut at = 0;
    for (i, ch) in text.char_indices() {
        bounds.push((at, i));
        at += ch.to_lowercase().map(char::len_utf8).sum::<usize>();
    }
    bounds.push((at, text.len()));
    debug_assert_eq!(at, lower.len());
    // The character of `text` that holds byte `at` of `lower`, or the next
    // one when `up` and `at` falls inside a character.
    let to_text = |at: usize, up: bool| match bounds.binary_search_by_key(&at, |&(l, _)| l) {
        Ok(k) => bounds[k].1,
        Err(k) if up => bounds[k.min(bounds.len() - 1)].1,
        Err(k) => bounds[k - 1].1,
    };

    let mut ranges = Vec::new();
    let mut from = 0;
    while let Some(found) = lower[from..].find(needle) {
        let start = from + found;
        ranges.push(to_text(start, false)..to_text(start + needle.len(), true));
        // Matches may overlap: carry on from the next character.
        from = start + lower[start..].chars().next().map_or(1, char::len_utf8);
    }
    ranges
}

/// One line of a search result, cut to `width` columns with `…`. Each part is
/// `(text, style, searched)`, and the first match of `needle` in a searched
/// part that fits in `width` is drawn bold.
fn hit_line(parts: &[(&str, Style, bool)], needle: &str, width: usize) -> Vec<Span<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let full: String = parts.iter().map(|&(text, ..)| text).collect();
    let cut = width_of(&full) > width;
    let shown = if cut {
        head(&full, width - 1).len()
    } else {
        full.len()
    };

    let mut mark: Option<Range<usize>> = None;
    let mut start = 0;
    for &(text, _, searched) in parts {
        if mark.is_none() && searched {
            mark = match_ranges(text, needle)
                .into_iter()
                .map(|r| start + r.start..start + r.end)
                .find(|r| r.end <= shown);
        }
        start += text.len();
    }

    let mut spans = Vec::new();
    let mut start = 0;
    for &(text, style, _) in parts {
        let (lo, hi) = (start.min(shown), (start + text.len()).min(shown));
        let mut cuts = vec![lo, hi];
        if let Some(m) = &mark {
            cuts.extend([m.start, m.end].into_iter().filter(|&b| lo < b && b < hi));
        }
        cuts.sort_unstable();
        for pair in cuts.windows(2) {
            let piece = pair[0]..pair[1];
            if piece.is_empty() {
                continue;
            }
            let bold = mark
                .as_ref()
                .is_some_and(|m| m.start <= piece.start && piece.end <= m.end);
            let style = if bold {
                style.fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                style
            };
            spans.push(Span::styled(full[piece].to_owned(), style));
        }
        start += text.len();
    }
    if cut {
        spans.push(Span::styled("…", Style::new().fg(DIM)));
    }
    spans
}

/// Where an excerpt's context starts, as a byte offset into `text`: the
/// earliest word start whose text up to `at` fits in `budget` columns, so no
/// word is cut at the start. When no word start fits (a word, or CJK text
/// without spaces, is wider than `budget`), as much of the end as fits.
fn excerpt_start(text: &str, at: usize, budget: usize) -> usize {
    let before = &text[..at];
    let word_start = |i: usize| i == 0 || before[..i].ends_with(char::is_whitespace);
    // A match that starts a word needs no context to stay whole.
    let mut start = word_start(at).then_some(at);
    let mut fallback = at;
    for (i, _) in before.char_indices().rev() {
        if width_of(&before[i..]) > budget {
            break;
        }
        fallback = i;
        if word_start(i) && !before[i..].starts_with(char::is_whitespace) {
            start = Some(i);
        }
    }
    let start = start.unwrap_or(fallback);
    // Never lead with a space.
    at - before[start..].trim_start().len()
}

/// A post's body on one line, cut with `…`. When that hides the body's first
/// match, an excerpt instead: `…`, a few words before the match, the match,
/// then as much as fits.
fn body_line(body: &str, needle: &str, style: Style, width: usize) -> Vec<Span<'static>> {
    let line = hit_line(&[(body, style, true)], needle, width);
    let Some(first) = match_ranges(body, needle).into_iter().next() else {
        return line;
    };
    let shown = if width_of(body) > width {
        head(body, width.saturating_sub(1)).len()
    } else {
        body.len()
    };
    if first.end <= shown || width == 0 {
        return line;
    }
    // The room after the leading `…`. The context gets a third of it, or,
    // when the rest of the body fits after it, whatever the rest leaves, if
    // that's more. Otherwise the end is cut with `…`.
    let room = width - 1;
    let rest = width_of(&body[first.start..]);
    let cut_budget = (room / 3).min(room.saturating_sub(width_of(&body[first.clone()]) + 1));
    let budget = if rest <= room {
        cut_budget.max(room - rest)
    } else {
        cut_budget
    };
    let start = excerpt_start(body, first.start, budget);
    let mut spans = vec![Span::styled("…", Style::new().fg(DIM))];
    spans.extend(hit_line(&[(&body[start..], style, true)], needle, room));
    spans
}

/// The query keeps at least this many columns (or all of a shorter query)
/// before the result count shows.
const QUERY_MIN_WIDTH: usize = 10;

/// The search box's first row: `⌕ `, the end of the query and the cursor,
/// then `N results` on the right. The count goes first when the row is too
/// narrow, so the end of the query, which was just typed, stays in view.
fn search_input(query: &str, results: usize, width: usize) -> Line<'static> {
    // The icon and the cursor.
    let fixed = 3;
    let keep = width_of(query).min(QUERY_MIN_WIDTH);
    let label = Some(results)
        .filter(|&n| n > 0)
        .map(|n| count(u32::try_from(n).unwrap_or(u32::MAX), "result"))
        .filter(|label| fixed + keep + 2 + width_of(label) <= width);
    let room = width.saturating_sub(fixed + label.as_deref().map_or(0, |l| width_of(l) + 2));
    let left = vec![
        Span::styled("⌕ ", Style::new().fg(LIGHT_BLUE)),
        Span::styled(tail(query, room).to_owned(), Style::new().fg(Color::White)),
        Span::styled("▏", Style::new().fg(LIGHT_BLUE)),
    ];
    match label {
        Some(label) => spread(left, vec![Span::styled(label, Style::new().fg(DIM))], width),
        None => Line::from(left),
    }
}

fn search_group(hit: SearchHit) -> &'static str {
    match hit {
        SearchHit::Invitation(_) | SearchHit::Suggestion(_) => "People",
        SearchHit::Job(_) => "Jobs",
        SearchHit::Post(_) => "Posts",
    }
}

/// A result's two lines. Only bold text is a match, so nothing else is bold.
fn hit_lines(app: &App, hit: SearchHit, needle: &str, width: usize) -> Vec<Line<'static>> {
    let white = Style::new().fg(Color::White);
    let gray = Style::new().fg(Color::Gray);
    let dim = Style::new().fg(DIM);
    let person = |name: &str, headline: &str, label: &'static str| {
        let label = format!(" · {label}");
        let mut second = hit_line(
            &[(headline, dim, true)],
            needle,
            width.saturating_sub(width_of(&label)),
        );
        second.push(Span::styled(label, Style::new().fg(LIGHT_BLUE)));
        vec![
            Line::from(hit_line(&[(name, white, true)], needle, width)),
            Line::from(second),
        ]
    };
    match hit {
        SearchHit::Invitation(i) => {
            let p = &app.invitations[i];
            person(p.name, p.headline, "Invitation")
        }
        SearchHit::Suggestion(i) => {
            let s = &app.suggestions[i];
            let label = if s.pending { "Pending" } else { "Connect" };
            person(s.person.name, s.person.headline, label)
        }
        SearchHit::Job(i) => {
            let job = &app.jobs[i];
            vec![
                Line::from(hit_line(
                    &[(job.title, Style::new().fg(LIGHT_BLUE), true)],
                    needle,
                    width,
                )),
                Line::from(hit_line(
                    &[
                        (job.company, white, true),
                        (" · ", dim, false),
                        (job.location, gray, true),
                        (" · ", dim, false),
                        (job.workplace.label(), dim, false),
                    ],
                    needle,
                    width,
                )),
            ]
        }
        SearchHit::Post(i) => {
            let post = &app.posts[i];
            // Same length as the body, so match offsets carry over.
            let body = post.body.replace('\n', " ");
            vec![
                Line::from(hit_line(
                    &[
                        (post.author, white, true),
                        (" · ", dim, false),
                        (post.headline, dim, true),
                    ],
                    needle,
                    width,
                )),
                Line::from(body_line(&body, needle, gray, width)),
            ]
        }
    }
}

fn render_search(frame: &mut Frame, app: &App, search: &Search, body: Rect) {
    let popup = search_area(body);
    frame.render_widget(Clear, popup);
    let block = card(" Search ", true);
    let inner = block.inner(popup);
    let width = usize_of(inner.width);
    frame.render_widget(block, popup);
    let [input, rule, results] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);

    let needle = search_needle(&search.query);
    let hits = app.search_hits(&search.query);
    frame.render_widget(search_input(&search.query, hits.len(), width), input);
    frame.render_widget(
        Line::styled("─".repeat(width), Style::new().fg(BORDER)),
        rule,
    );
    if hits.is_empty() {
        let text = if needle.is_empty() {
            SEARCH_PROMPT
        } else {
            NO_RESULTS
        };
        frame.render_widget(
            Line::styled(truncate(text, width), Style::new().fg(DIM)),
            results,
        );
        return;
    }

    // One column goes to the selection bar.
    let width = width.saturating_sub(1);
    let mut items = Vec::new();
    let mut selected = None;
    let mut group = None;
    for (i, &hit) in hits.iter().enumerate() {
        let heading = search_group(hit);
        if group != Some(heading) {
            let mut lines = Vec::new();
            if group.is_some() {
                lines.push(Line::raw(""));
            }
            lines.push(Line::styled(heading, Style::new().fg(LIGHT_BLUE)));
            items.push(lines);
            group = Some(heading);
        }
        if i == search.selected {
            selected = Some(items.len());
        }
        items.push(hit_lines(app, hit, &needle, width));
    }
    render_list(frame, results, items, selected);
}

// ---------------------------------------------------------- My Network

fn render_network(frame: &mut Frame, app: &App, area: Rect) {
    let show_side = area.width >= PROFILE_MIN_WIDTH;
    let [left, main] = Layout::horizontal([
        Constraint::Length(if show_side { PROFILE_WIDTH } else { 0 }),
        Constraint::Min(0),
    ])
    .areas(area);

    if show_side {
        let block = card(" Manage my network ", false);
        let width = usize_of(block.inner(left).width);
        let pending = app.suggestions.iter().filter(|s| s.pending).count();
        let lines = vec![
            stat_line(
                "Connections",
                usize::try_from(app.profile.connections).unwrap_or(usize::MAX),
                width,
            ),
            stat_line("Invitations", app.invitations.len(), width),
            stat_line("Sent", pending, width),
        ];
        let height = u16_of(lines.len() + 2).min(left.height);
        frame.render_widget(Paragraph::new(lines).block(block), Rect { height, ..left });
    }

    let wanted = u16_of((app.invitations.len() * 2).max(1) + 2);
    let [inv_area, sug_area] = Layout::vertical([
        Constraint::Length(wanted.min((main.height / 2).max(3))),
        Constraint::Min(0),
    ])
    .areas(main);
    let (inv_sel, sug_sel) = match app.network_item() {
        Some(NetworkItem::Invitation(i)) => (Some(i), None),
        Some(NetworkItem::Suggestion(i)) => (None, Some(i)),
        None => (None, None),
    };

    let title = match app.invitations.len() {
        0 => " Invitations ".to_owned(),
        n => format!(" Invitations ({n}) "),
    };
    let block = card(&title, inv_sel.is_some());
    let inner = block.inner(inv_area);
    frame.render_widget(block, inv_area);
    // One column goes to the selection bar.
    let width = usize_of(inner.width.saturating_sub(1));
    if app.invitations.is_empty() {
        frame.render_widget(
            Line::styled("No pending invitations", Style::new().fg(DIM)),
            inner,
        );
    } else {
        let buttons = || {
            vec![
                Span::styled("Ignore", Style::new().fg(DIM)),
                Span::raw("  "),
                Span::styled(" Accept ", Style::new().fg(Color::White).bg(BLUE).bold()),
            ]
        };
        let items = app
            .invitations
            .iter()
            .map(|p| {
                let name = truncate(p.name, width.saturating_sub(18));
                vec![
                    spread(
                        vec![Span::styled(name, Style::new().fg(Color::White).bold())],
                        buttons(),
                        width,
                    ),
                    Line::styled(
                        truncate(&format!("{} · {} mutual", p.headline, p.mutual), width),
                        Style::new().fg(DIM),
                    ),
                ]
            })
            .collect();
        render_list(frame, inner, items, inv_sel);
    }

    let block = card(" People you may know ", sug_sel.is_some());
    let inner = block.inner(sug_area);
    frame.render_widget(block, sug_area);
    let width = usize_of(inner.width.saturating_sub(1));
    let items = app
        .suggestions
        .iter()
        .map(|s| {
            let button = if s.pending {
                Span::styled("◷ Pending", Style::new().fg(DIM))
            } else {
                Span::styled("+ Connect", Style::new().fg(LIGHT_BLUE).bold())
            };
            let name = truncate(s.person.name, width.saturating_sub(11));
            vec![
                spread(
                    vec![Span::styled(name, Style::new().fg(Color::White).bold())],
                    vec![button],
                    width,
                ),
                Line::styled(
                    truncate(
                        &format!("{} · {} mutual", s.person.headline, s.person.mutual),
                        width,
                    ),
                    Style::new().fg(DIM),
                ),
            ]
        })
        .collect();
    render_list(frame, inner, items, sug_sel);
}

// ---------------------------------------------------------------- Jobs

fn render_jobs(frame: &mut Frame, app: &App, area: Rect) {
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(area);

    let block = card(" Jobs for you ", false);
    let inner = block.inner(list_area);
    frame.render_widget(block, list_area);
    let width = usize_of(inner.width.saturating_sub(1));
    let items = app
        .jobs
        .iter()
        .map(|job| {
            let star = if job.saved { " ★" } else { "" };
            let mut place = vec![Span::styled(
                format!("{} · {}", job.location, job.workplace.label()),
                Style::new().fg(DIM),
            )];
            if job.applied {
                place.push(Span::styled("  Applied", Style::new().fg(GREEN).bold()));
            }
            vec![
                Line::from(vec![
                    Span::styled(
                        truncate(job.title, width.saturating_sub(2)),
                        Style::new().fg(LIGHT_BLUE).bold(),
                    ),
                    Span::styled(star, Style::new().fg(LIGHT_BLUE)),
                ]),
                Line::styled(truncate(job.company, width), Style::new().fg(Color::White)),
                Line::from(place),
            ]
        })
        .collect();
    render_list(frame, inner, items, Some(app.job_sel));

    let Some(job) = app.jobs.get(app.job_sel) else {
        return;
    };
    let block = card(" Job details ", false);
    let inner = block.inner(detail_area);
    let width = usize_of(inner.width);
    frame.render_widget(block, detail_area);

    let mut lines: Vec<Line> = wrap(job.title, width)
        .into_iter()
        .map(|l| Line::styled(l, Style::new().fg(Color::White).bold()))
        .collect();
    lines.push(Line::raw(job.company));
    lines.push(Line::from(vec![
        Span::styled(job.location, Style::new().fg(Color::Gray)),
        Span::styled(" · ", Style::new().fg(DIM)),
        Span::styled(job.workplace.label(), Style::new().fg(Color::Gray).bold()),
    ]));
    lines.push(Line::styled(
        format!(
            "Posted {} ago · {}",
            age(job.days_ago * 24),
            count(job.applicants, "applicant")
        ),
        Style::new().fg(DIM),
    ));
    lines.push(Line::raw(""));
    let apply = if job.applied {
        Span::styled("✔ Applied", Style::new().fg(GREEN).bold())
    } else {
        Span::styled(
            " Easy Apply ",
            Style::new().fg(Color::White).bg(BLUE).bold(),
        )
    };
    let save = if job.saved {
        Span::styled("★ Saved", Style::new().fg(LIGHT_BLUE).bold())
    } else {
        Span::styled("☆ Save", Style::new().fg(LIGHT_BLUE))
    };
    lines.push(Line::from(vec![apply, Span::raw("   "), save]));
    lines.push(Line::raw(""));
    lines.push(Line::styled("About the job", Style::new().bold()));
    lines.extend(
        wrap(&job.description.replace('\n', "\n\n"), width)
            .into_iter()
            .map(|l| Line::styled(l, Style::new().fg(Color::Gray))),
    );
    frame.render_widget(Paragraph::new(lines), inner);
}

// ----------------------------------------------------------- Messaging

fn render_messaging(frame: &mut Frame, app: &App, area: Rect) {
    let [list_area, thread_area] = Layout::horizontal([
        Constraint::Length(MESSAGING_LIST_WIDTH.min(area.width / 2)),
        Constraint::Min(0),
    ])
    .areas(area);

    let block = card(" Messaging ", app.mode == Mode::Browse);
    let inner = block.inner(list_area);
    frame.render_widget(block, list_area);
    let width = usize_of(inner.width.saturating_sub(1));
    let items = app
        .conversations
        .iter()
        .map(|chat| {
            let last = chat.messages.last();
            let when = last.map_or_else(String::new, |m| age(m.hours_ago));
            let (dot, name_style, preview_style) = if chat.unread {
                (
                    "● ",
                    Style::new().fg(Color::White).bold(),
                    Style::new().fg(Color::White),
                )
            } else {
                ("  ", Style::new().fg(Color::Gray), Style::new().fg(DIM))
            };
            let preview = last.map_or_else(String::new, |m| {
                let who = if m.from_me { "You: " } else { "" };
                format!("{who}{}", m.body)
            });
            let name = truncate(chat.name, width.saturating_sub(2 + width_of(&when) + 1));
            vec![
                spread(
                    vec![
                        Span::styled(dot, Style::new().fg(LIGHT_BLUE)),
                        Span::styled(name, name_style),
                    ],
                    vec![Span::styled(when, Style::new().fg(DIM))],
                    width,
                ),
                Line::styled(
                    format!("  {}", truncate(&preview, width.saturating_sub(2))),
                    preview_style,
                ),
            ]
        })
        .collect();
    render_list(frame, inner, items, Some(app.chat_sel));

    let Some(chat) = app.open_chat.and_then(|i| app.conversations.get(i)) else {
        let block = card(" Conversation ", false);
        let inner = block.inner(thread_area);
        frame.render_widget(block, thread_area);
        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(2),
            Constraint::Fill(1),
        ])
        .areas(inner);
        frame.render_widget(
            Paragraph::new(vec![
                Line::raw("Select a conversation"),
                Line::raw("and press enter to open it"),
            ])
            .fg(DIM)
            .centered(),
            middle,
        );
        return;
    };

    let title = format!(" {} ", chat.name);
    let block = card(&title, false);
    let inner = block.inner(thread_area);
    frame.render_widget(block, thread_area);
    let width = usize_of(inner.width);
    let [head, rule, messages, reply] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .areas(inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(chat.name, Style::new().fg(Color::White).bold()),
            Line::styled(truncate(chat.headline, width), Style::new().fg(DIM)),
        ]),
        head,
    );
    frame.render_widget(
        Line::styled("─".repeat(width), Style::new().fg(BORDER)),
        rule,
    );

    let mut lines = Vec::new();
    for m in &chat.messages {
        if !lines.is_empty() {
            lines.push(Line::raw(""));
        }
        let who = if m.from_me {
            app.profile.name
        } else {
            chat.name
        };
        lines.push(author_line(who, m.hours_ago, width));
        lines.extend(
            wrap(&m.body, width.saturating_sub(2))
                .into_iter()
                .map(|l| Line::styled(format!("  {l}"), Style::new().fg(Color::Gray))),
        );
    }
    // Newest messages sit at the bottom, right above the reply box.
    let scroll = lines.len().saturating_sub(usize_of(messages.height));
    frame.render_widget(Paragraph::new(lines).scroll((u16_of(scroll), 0)), messages);

    let focused = app.mode == Mode::Reply;
    let block = card(" Reply ", focused);
    let box_width = usize_of(block.inner(reply).width);
    let cursor = Span::styled("▏", Style::new().fg(LIGHT_BLUE));
    let line = match (focused, app.reply.is_empty()) {
        (false, _) => Line::styled("Press enter to reply", Style::new().fg(DIM)),
        (true, true) => Line::from(vec![
            cursor,
            Span::styled("Write a message…", Style::new().fg(DIM)),
        ]),
        (true, false) => {
            // Show the end of a long reply, where the cursor is. One column
            // goes to the cursor.
            let shown = tail(&app.reply, box_width.saturating_sub(1)).to_owned();
            Line::from(vec![Span::raw(shown), cursor])
        }
    };
    frame.render_widget(Paragraph::new(line).block(block), reply);
}

// ------------------------------------------------------- Notifications

fn render_notifications(frame: &mut Frame, app: &App, area: Rect) {
    let block = card(" Notifications ", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let width = usize_of(inner.width.saturating_sub(1));
    let items = app
        .notifications
        .iter()
        .map(|n| {
            // Opening the tab clears `unread` at once, so `is_new` is what
            // keeps this visit's new ones marked.
            let (dot, text_style) = if n.is_new || n.unread {
                ("● ", Style::new().fg(Color::White).bold())
            } else {
                ("  ", Style::new().fg(Color::Gray))
            };
            vec![
                Line::from(vec![
                    Span::styled(dot, Style::new().fg(LIGHT_BLUE)),
                    Span::styled(truncate(n.text, width.saturating_sub(2)), text_style),
                ]),
                Line::styled(format!("  {} ago", age(n.hours_ago)), Style::new().fg(DIM)),
            ]
        })
        .collect();
    render_list(frame, inner, items, Some(app.notif_sel));
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::crossterm::event::KeyCode;
    use ratatui::crossterm::event::KeyEvent;
    use ratatui::crossterm::event::KeyModifiers;

    use super::*;

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_str(app: &mut App, s: &str) {
        for ch in s.chars() {
            press(app, KeyCode::Char(ch));
        }
    }

    fn buffer(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test backend is infallible");
        terminal
            .draw(|f| draw(f, app))
            .expect("test backend is infallible");
        terminal.backend().buffer().clone()
    }

    fn screen(app: &App, width: u16, height: u16) -> String {
        let buf = buffer(app, width, height);
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The column where `needle` first appears on screen.
    fn column_of(s: &str, needle: &str) -> Option<usize> {
        s.lines()
            .find_map(|line| line.find(needle).map(|byte| line[..byte].chars().count()))
    }

    /// One app per screen and mode, so tests can cover every layout.
    fn every_state() -> Vec<(&'static str, App)> {
        let at = |keys: &[KeyCode], text: &str| {
            let mut app = App::new();
            for &k in keys {
                press(&mut app, k);
            }
            type_str(&mut app, text);
            app
        };
        let mut toast = at(&[KeyCode::Char('3'), KeyCode::Char('e')], "");
        toast.status = Some("Application sent");
        let comment = |text: &str| {
            at(
                &[KeyCode::Char('j'), KeyCode::Enter, KeyCode::Char('c')],
                text,
            )
        };
        let mut commented = comment("Agreed, the meeting-to-doc rule works");
        press(&mut commented, KeyCode::Enter);
        vec![
            ("home", at(&[], "")),
            ("detail", at(&[KeyCode::Char('j'), KeyCode::Enter], "")),
            ("comment box", comment("A comment in progress")),
            ("comment box empty", comment("")),
            ("detail commented", commented),
            (
                "composer",
                at(
                    &[KeyCode::Char('p')],
                    "A long draft that wraps over several rows ",
                ),
            ),
            ("composer empty", at(&[KeyCode::Char('p')], "")),
            ("network", at(&[KeyCode::Char('2')], "")),
            (
                "network suggestions",
                at(&[KeyCode::Char('2'), KeyCode::End, KeyCode::Char('j')], ""),
            ),
            ("jobs", at(&[KeyCode::Char('3'), KeyCode::Char('s')], "")),
            ("jobs toast", toast),
            ("messaging", at(&[KeyCode::Char('4')], "")),
            (
                "reply",
                at(&[KeyCode::Char('4'), KeyCode::Enter], "a reply in progress"),
            ),
            ("notifications", at(&[KeyCode::Char('5')], "")),
            ("search", at(&[KeyCode::Char('/')], "bright")),
            ("search empty", at(&[KeyCode::Char('/')], "")),
            ("search no match", at(&[KeyCode::Char('/')], "zqzq")),
            (
                "search over post",
                at(
                    &[
                        KeyCode::Char('j'),
                        KeyCode::Enter,
                        KeyCode::Char('j'),
                        KeyCode::Char('/'),
                    ],
                    "hana",
                ),
            ),
        ]
    }

    /// The inside of the post view's Post card on a `width`×`height` screen.
    fn post_inner(width: u16, height: u16) -> Rect {
        let body = screen_areas(Rect::new(0, 0, width, height))[1];
        card(" Post ", true).inner(home_columns(body)[1])
    }

    /// Where the comment box is on a `width`×`height` screen, if it shows.
    fn comment_box(width: u16, height: u16) -> Option<Rect> {
        post_detail_areas(post_inner(width, height)).1
    }

    /// Each row of `area` starts and ends with its border and is exactly as
    /// wide as the box, measured by display width.
    fn assert_bordered(buf: &Buffer, area: Rect, what: &str) {
        for y in area.top()..area.bottom() {
            let row = text_between(buf, y, area.left(), area.right());
            let ends = match y {
                y if y == area.top() => ('╭', '╮'),
                y if y == area.bottom() - 1 => ('╰', '╯'),
                _ => ('│', '│'),
            };
            assert!(
                row.starts_with(ends.0)
                    && row.ends_with(ends.1)
                    && width_of(&row) == usize_of(area.width),
                "{what}, row {y}: {row:?}"
            );
        }
    }

    /// The text inside the comment box's border, trimmed.
    fn comment_box_text(app: &App, width: u16, height: u16) -> String {
        let area = comment_box(width, height).expect("the box shows");
        let buf = buffer(app, width, height);
        assert_bordered(&buf, area, &format!("{width}x{height}"));
        text_between(&buf, area.y + 1, area.left() + 1, area.right() - 1)
            .trim()
            .to_owned()
    }

    fn post_view(down: usize) -> App {
        let mut app = App::new();
        for _ in 0..down {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Enter);
        app
    }

    fn comment_on(app: &mut App, text: &str) {
        press(app, KeyCode::Char('c'));
        type_str(app, text);
        press(app, KeyCode::Enter);
    }

    #[test]
    fn the_comment_box_shows_each_state() {
        let mut app = post_view(0);
        for (width, height) in [(120, 30), (80, 24), (60, 20)] {
            assert_eq!(comment_box_text(&app, width, height), "Press c to comment");
        }
        let area = comment_box(120, 30).expect("the box shows");
        let border = |app: &App| buffer(app, 120, 30)[(area.x, area.y)].fg;
        assert_eq!(border(&app), BORDER);
        // The unfocused box is dim, like the reply box.
        let buf = buffer(&app, 120, 30);
        assert_eq!(buf[(area.x + 2, area.y + 1)].fg, DIM);

        press(&mut app, KeyCode::Char('c'));
        for (width, height) in [(120, 30), (80, 24), (60, 20)] {
            assert_eq!(comment_box_text(&app, width, height), "Add a comment…▏");
        }
        assert_eq!(border(&app), LIGHT_BLUE, "the focused box's color");

        type_str(&mut app, "Nice one");
        assert_eq!(comment_box_text(&app, 120, 30), "Nice one▏");
        assert_eq!(border(&app), LIGHT_BLUE);

        press(&mut app, KeyCode::Esc);
        assert_eq!(comment_box_text(&app, 120, 30), "Press c to comment");
        assert_eq!(border(&app), BORDER);
    }

    #[test]
    fn a_long_comment_draft_shows_its_end() {
        let draft = WIDE.repeat(4);
        let mut app = post_view(0);
        press(&mut app, KeyCode::Char('c'));
        type_str(&mut app, &draft);
        for (width, height) in [(120, 30), (80, 24), (60, 20)] {
            let area = comment_box(width, height).expect("the box shows");
            assert!(
                width_of(&draft) > usize_of(area.width),
                "the draft must not fit"
            );
            let text = comment_box_text(&app, width, height);
            let shown = text.strip_suffix('▏').expect("the cursor at the end");
            assert!(draft.ends_with(shown), "{width}x{height}: {shown:?}");
            // A border and a padding column each side, and the cursor.
            let room = usize_of(area.width) - 5;
            assert!(
                width_of(shown) <= room && width_of(shown) + 1 >= room,
                "{width}x{height}: {shown:?} should fill {room} columns, less at most one"
            );
        }
    }

    #[test]
    fn a_posted_comment_raises_the_count_everywhere() {
        let mut app = post_view(0);
        let reactions = app.posts[0].reactions;
        let before = format!("{reactions} reactions · 3 comments");
        assert!(screen(&app, 120, 30).contains(&before));

        comment_on(&mut app, "  Congrats to the whole team  ");
        let after = format!("{reactions} reactions · 4 comments");
        let s = screen(&app, 120, 30);
        assert!(s.contains("Comment posted"), "{s}");
        assert!(s.contains(&after), "post view: {s}");
        assert!(s.contains("┃ Jordan Vale · now"), "selected: {s}");
        assert!(s.contains("┃ Congrats to the whole team"), "{s}");

        press(&mut app, KeyCode::Esc);
        let s = screen(&app, 120, 30);
        assert!(s.contains(&after), "feed card: {s}");
        assert!(!s.contains(&before), "{s}");
    }

    #[test]
    fn deleting_a_comment_lowers_the_count_everywhere() {
        let mut app = post_view(0);
        let reactions = app.posts[0].reactions;
        comment_on(&mut app, "Posted by mistake");
        assert!(screen(&app, 120, 30).contains(&format!("{reactions} reactions · 4 comments")));

        press(&mut app, KeyCode::Char('x'));
        let three = format!("{reactions} reactions · 3 comments");
        let s = screen(&app, 120, 30);
        assert!(s.contains("Comment deleted"), "{s}");
        assert!(s.contains(&three), "post view: {s}");
        assert!(!s.contains("Posted by mistake"), "{s}");
        // The cursor moved up to the last seed comment.
        assert!(s.contains("┃ Hana Sorensen · now"), "{s}");

        press(&mut app, KeyCode::Esc);
        let s = screen(&app, 120, 30);
        assert!(
            s.contains(&three) && !s.contains("4 comments"),
            "feed card: {s}"
        );

        // The only comment on Lucia Brandt's post: back to No comments yet.
        let mut app = post_view(7);
        comment_on(&mut app, "Floor three thanks you");
        press(&mut app, KeyCode::Char('x'));
        let s = screen(&app, 120, 30);
        assert!(s.contains("No comments yet"), "{s}");
        assert!(!s.contains("│ Comments "), "{s}");
        assert!(s.contains("· 0 comments"), "{s}");
        press(&mut app, KeyCode::Esc);
        assert!(screen(&app, 120, 30).contains("· 0 comments"));

        // A seed comment stays, with no toast.
        let mut app = post_view(0);
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('x'));
        let s = screen(&app, 120, 30);
        assert!(s.contains(&three), "{s}");
        let footer = s.lines().last().expect("screen has rows").trim();
        assert!(
            footer.starts_with("j/k comments") && !footer.contains("Comment deleted"),
            "no toast: {s}"
        );
    }

    #[test]
    fn the_first_comment_replaces_no_comments_yet() {
        let mut app = post_view(7);
        assert_eq!(app.posts[7].author, "Lucia Brandt");
        let s = screen(&app, 120, 30);
        assert!(s.contains("No comments yet"), "{s}");
        assert!(s.contains("· 0 comments"), "{s}");
        assert!(!s.contains("Comments "), "{s}");

        comment_on(&mut app, "Floor three thanks you");
        let s = screen(&app, 120, 30);
        assert!(!s.contains("No comments yet"), "{s}");
        assert!(s.contains("│ Comments "), "the heading: {s}");
        assert!(s.contains("┃ Floor three thanks you"), "{s}");
        assert!(
            s.contains("· 1 comment ") && !s.contains("1 comments"),
            "{s}"
        );

        press(&mut app, KeyCode::Esc);
        let s = screen(&app, 120, 30);
        assert!(
            s.contains("· 1 comment ") && !s.contains("1 comments"),
            "feed: {s}"
        );
    }

    /// Maren Holt's post with three long comments of yours, the last one mixing
    /// in double-width text.
    fn long_comments() -> (App, Vec<String>) {
        let mut app = post_view(0);
        let bodies: Vec<String> = (1..=3)
            .map(|n| {
                let tail = if n == 3 { WIDE } else { "and that is that." };
                format!(
                    "Comment {n}: {}{tail}",
                    "the caching write-up was great and I learned a lot from it, ".repeat(3)
                )
            })
            .collect();
        for body in &bodies {
            comment_on(&mut app, body);
        }
        (app, bodies)
    }

    #[test]
    fn the_newest_long_comment_shows_whole_above_the_box() {
        let (app, bodies) = long_comments();
        let newest = bodies.last().expect("three bodies");
        assert_eq!(app.mode, Mode::PostDetail { comment: 5 });
        assert_eq!(&app.posts[0].comments[5].body, newest);
        for (width, height) in [(120, 30), (60, 20)] {
            let buf = buffer(&app, width, height);
            let inner = post_inner(width, height);
            let area = comment_box(width, height).expect("the box shows");
            assert_bordered(&buf, area, &format!("{width}x{height}"));
            // The selected comment's rows, all above the box.
            let rows: Vec<String> = (inner.top()..area.top())
                .map(|y| text_between(&buf, y, inner.left(), inner.right()))
                .filter_map(|row| row.strip_prefix("┃ ").map(|r| r.trim_end().to_owned()))
                .collect();
            assert!(rows.len() > 4, "{width}x{height}: wraps: {rows:#?}");
            assert_eq!(rows[0], "Jordan Vale · now", "{width}x{height}: {rows:#?}");
            assert_eq!(rows[1], app.profile.headline, "{width}x{height}");
            assert_eq!(&rows[2..].join(" "), newest, "{width}x{height}: {rows:#?}");
            // It ends right above the box.
            let last = text_between(&buf, area.top() - 1, inner.left(), inner.right());
            assert!(last.starts_with("┃ "), "{width}x{height}: {last:?}");
        }
    }

    #[test]
    fn the_post_view_stays_inside_its_borders_at_every_size() {
        let long: String = "日本語 text".repeat(375);
        assert_eq!(long.chars().count(), 3_000);
        let focused = |text: &str| {
            let mut app = post_view(0);
            press(&mut app, KeyCode::Char('c'));
            type_str(&mut app, text);
            app
        };
        let (comments, _) = long_comments();
        let (mut comments_focused, _) = long_comments();
        press(&mut comments_focused, KeyCode::Char('c'));
        type_str(&mut comments_focused, "more");
        let states = [
            ("unfocused", post_view(0)),
            ("focused empty", focused("")),
            ("focused short", focused("Nice one")),
            ("focused long", focused(&long)),
            ("long comments", comments),
            ("long comments focused", comments_focused),
        ];
        std::thread::scope(|scope| {
            for (name, app) in &states {
                scope.spawn(move || {
                    let mut terminal =
                        Terminal::new(TestBackend::new(1, 1)).expect("test backend is infallible");
                    for width in 1..=140 {
                        for height in 1..=40 {
                            terminal.backend_mut().resize(width, height);
                            terminal
                                .draw(|f| draw(f, app))
                                .unwrap_or_else(|e| panic!("{name} at {width}x{height}: {e}"));
                            if let Some(area) = comment_box(width, height) {
                                let buf = terminal.backend().buffer();
                                assert_bordered(buf, area, &format!("{name} at {width}x{height}"));
                            }
                        }
                    }
                });
            }
        });

        for (name, app) in &states {
            // In full at these sizes.
            for (width, height) in [(120, 30), (80, 24), (60, 20)] {
                let area = comment_box(width, height).expect("the box shows");
                assert_eq!(area.height, COMMENT_BOX_HEIGHT);
                assert!(!comment_box_text(app, width, height).is_empty(), "{name}");
            }
            // The header keeps the logo, every tab and every badge.
            for width in [140, 120, 80, 70, 60] {
                let row = header_row(app, width, 30);
                assert!(row.starts_with("│ in "), "{name} at {width}: {row:?}");
                let (numbers, badges) = tabs_and_badges(&row);
                assert_eq!(numbers, "12345", "{name} at {width}: {row:?}");
                assert_eq!(badges, ["3", "2", "3"], "{name} at {width}: {row:?}");
            }
        }

        // The long draft shows its end.
        let text = comment_box_text(&states[3].1, 120, 30);
        let shown = text.strip_suffix('▏').expect("the cursor at the end");
        assert!(width_of(shown) > 50 && long.ends_with(shown), "{shown:?}");
    }

    #[test]
    fn comment_hints_and_labels() {
        let footer = |app: &App| {
            screen(app, 80, 24)
                .lines()
                .last()
                .expect("screen has rows")
                .trim()
                .to_owned()
        };
        let mut app = post_view(0);
        assert_eq!(
            footer(&app),
            "j/k comments  l like  c comment  x delete  esc back  1-5 tabs  q quit"
        );
        assert!(hints_line(hints(&app)).width() <= 80);
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(
            footer(&app),
            "type write  backspace delete  enter post  esc cancel"
        );
        assert!(hints_line(hints(&app)).width() <= 80);
        for (key, what) in hints(&app) {
            assert_single_width("comment hints", key);
            assert_single_width("comment hints", what);
        }
        for label in [
            " Add a comment ",
            "Press c to comment",
            "Add a comment…",
            "Comment posted",
            "Comment deleted",
            "Write something first",
            "▏",
            "┃ ",
        ] {
            assert_single_width("comment box", label);
        }
    }

    #[test]
    fn helpers() {
        assert_eq!(age(0), "now");
        assert_eq!(age(23), "23h");
        assert_eq!(age(50), "2d");
        assert_eq!(age(400), "2w");
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(1_204), "1,204");
        assert_eq!(thousands(1_000_000), "1,000,000");
        assert_eq!(count(1, "reaction"), "1 reaction");
        assert_eq!(count(2_390, "reaction"), "2,390 reactions");
        assert_eq!(wrap("aa bb cc", 5), ["aa bb", "cc"]);
        assert_eq!(wrap("abcdefg", 3), ["abc", "def", "g"]);
        assert_eq!(wrap("a\nb", 5), ["a", "b"]);
        assert_eq!(hard_wrap("ab  cd", 3), ["ab ", " cd"]);
        assert_eq!(truncate("hello", 4), "hel…");
        assert_eq!(truncate("hi", 4), "hi");
        // Typed text can be double width.
        assert_eq!(width_of("日本"), 4);
        assert_eq!(head("日本語", 5), "日本");
        assert_eq!(tail("日本語", 5), "本語");
        assert_eq!(tail("ab日", 3), "b日");
        assert_eq!(hard_wrap("a日本語", 4), ["a日", "本語"]);
        assert_eq!(hard_wrap("日", 1), ["日"], "always makes progress");
        assert_eq!(hard_wrap("", 4), [""]);
        assert_eq!(wrap("日本語 abc", 6), ["日本語", "abc"]);
        assert_eq!(wrap("日本語です", 4), ["日本", "語で", "す"]);
        assert_eq!(truncate("日本語", 5), "日本…");
        assert_eq!(first_visible(&[3, 3, 3], 2, 6), 1);
        assert_eq!(
            first_visible(&[3, 3, 3], 0, 1),
            0,
            "never past the selection"
        );
        assert_eq!(first_visible(&[], 4, 10), 0);
    }

    #[test]
    fn home_at_120_shows_three_columns() {
        let app = App::new();
        let s = screen(&app, 120, 30);
        let profile = column_of(&s, "Connections").expect("profile card shown");
        let feed = column_of(&s, "Start a post").expect("composer box shown");
        let news = column_of(&s, "WorkedIn News").expect("sidebar shown");
        assert!(
            profile < feed && feed < news,
            "{profile} {feed} {news}\n{s}"
        );
        assert!(profile < usize::from(PROFILE_WIDTH));
        assert!(news >= 120 - usize::from(SIDEBAR_WIDTH));
        for needle in [
            "in",
            "Search",
            "1 Home",
            "Maren Holt",
            "Saved jobs",
            "Top stories",
        ] {
            assert!(s.contains(needle), "missing {needle:?} in:\n{s}");
        }
    }

    #[test]
    fn narrow_home_hides_side_columns() {
        let app = App::new();
        let s = screen(&app, 90, 30);
        assert!(!s.contains("WorkedIn News"), "{s}");
        assert!(s.contains("Connections"), "{s}");
        assert!(s.contains("Start a post"), "{s}");

        let s = screen(&app, 70, 30);
        assert!(!s.contains("WorkedIn News"), "{s}");
        assert!(!s.contains("Connections"), "{s}");
        assert!(s.contains("Start a post"), "{s}");

        // The exact thresholds.
        assert!(screen(&app, 100, 30).contains("WorkedIn News"));
        assert!(!screen(&app, 99, 30).contains("WorkedIn News"));
        assert!(screen(&app, 80, 30).contains("Connections"));
        assert!(!screen(&app, 79, 30).contains("Connections"));
    }

    #[test]
    fn every_size_renders_without_panicking() {
        // One thread per state: 11 × 5,600 frames is slow on a single core.
        std::thread::scope(|scope| {
            for (name, app) in every_state() {
                scope.spawn(move || {
                    let mut terminal =
                        Terminal::new(TestBackend::new(1, 1)).expect("test backend is infallible");
                    for width in 1..=140 {
                        for height in 1..=40 {
                            terminal.backend_mut().resize(width, height);
                            terminal
                                .draw(|f| draw(f, &app))
                                .unwrap_or_else(|e| panic!("{name} at {width}x{height}: {e}"));
                        }
                    }
                });
            }
        });
    }

    #[test]
    fn post_card_shows_counts_and_like_state() {
        let mut app = App::new();
        let post = &app.posts[0];
        let counts = format!(
            "{} reactions · {} comments",
            post.reactions,
            post.comments.len()
        );
        let s = screen(&app, 120, 30);
        assert!(s.contains(&counts), "missing {counts:?} in:\n{s}");
        assert!(s.contains("Like  Comment  Repost  Send"), "{s}");
        assert!(!s.contains("Liked"), "{s}");
        assert!(s.contains("Maren Holt · 2h"), "{s}");

        press(&mut app, KeyCode::Char('l'));
        let s = screen(&app, 120, 30);
        assert!(s.contains("Liked  Comment  Repost  Send"), "{s}");
        assert!(s.contains(&format!("{} reactions", app.posts[0].reactions)));
    }

    #[test]
    fn published_post_renders_first_with_now() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, "Shipped the terminal app");
        let s = screen(&app, 120, 30);
        assert!(s.contains("Create a post"), "{s}");
        assert!(s.contains("Shipped the terminal app"), "{s}");

        press(&mut app, KeyCode::Enter);
        let s = screen(&app, 120, 30);
        assert!(!s.contains("Create a post"), "{s}");
        assert!(s.contains("Post published"), "{s}");
        let mine = s.find("Jordan Vale · now").expect("new post shown");
        let theirs = s.find("Maren Holt · 2h").expect("old first post shown");
        assert!(mine < theirs, "{s}");
    }

    #[test]
    fn post_detail_shows_full_body_and_comments() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        let s = screen(&app, 120, 40);
        // The fifth item is past the card preview's three lines.
        assert!(s.contains("5. Bring snacks to design reviews."), "{s}");
        assert!(s.contains("Comments"), "{s}");
        assert!(s.contains("Number 5 is the real secret"), "{s}");

        press(&mut app, KeyCode::Esc);
        let s = screen(&app, 120, 40);
        assert!(!s.contains("Number 5 is the real secret"), "{s}");
    }

    #[test]
    fn badges_follow_counts() {
        let mut app = App::new();
        let s = screen(&app, 120, 30);
        for needle in ["2 My Network (3)", "4 Messaging (2)", "5 Notifications (3)"] {
            assert!(s.contains(needle), "missing {needle:?} in:\n{s}");
        }
        assert!(!s.contains("Home ("), "{s}");
        assert!(!s.contains("Jobs ("), "{s}");

        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('a'));
        assert!(screen(&app, 120, 30).contains("2 My Network (2)"));
        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Char('a'));
        let s = screen(&app, 120, 30);
        assert!(!s.contains("My Network ("), "hidden at 0:\n{s}");
        assert!(s.contains("No pending invitations"), "{s}");

        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        assert!(screen(&app, 120, 30).contains("4 Messaging (1)"));

        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('5'));
        let s = screen(&app, 120, 30);
        assert!(
            s.contains("5 Notifications") && !s.contains("Notifications ("),
            "{s}"
        );
    }

    #[test]
    fn profile_card_follows_saved_jobs_and_connections() {
        let mut app = App::new();
        let stat = |app: &App, label: &str| -> String {
            let s = screen(app, 120, 30);
            let line = s
                .lines()
                .find(|l| l.contains(label))
                .unwrap_or_else(|| panic!("missing {label:?} in:\n{s}"));
            let after = &line[line.find(label).expect("just found") + label.len()..];
            after
                .trim_start()
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .expect("split yields at least one item")
                .to_owned()
        };
        assert_eq!(stat(&app, "Saved jobs"), app.saved_jobs().to_string());
        let saved = app.saved_jobs();

        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('s'));
        assert!(screen(&app, 120, 30).contains("★ Saved"));
        press(&mut app, KeyCode::Char('1'));
        assert_eq!(stat(&app, "Saved jobs"), (saved + 1).to_string());

        assert_eq!(stat(&app, "Connections"), "312");
        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('1'));
        assert_eq!(stat(&app, "Connections"), "313");
    }

    #[test]
    fn network_lists_both_sections() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        let s = screen(&app, 120, 30);
        for needle in [
            "Invitations (3)",
            "People you may know",
            "Noor Haddad",
            "Mateo Achebe",
            "Accept",
            "+ Connect",
        ] {
            assert!(s.contains(needle), "missing {needle:?} in:\n{s}");
        }
        for _ in 0..3 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Char('c'));
        assert!(screen(&app, 120, 30).contains("◷ Pending"));
    }

    #[test]
    fn jobs_show_details_and_applied() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('j'));
        let s = screen(&app, 120, 30);
        for needle in [
            "Terminal UI Designer",
            "Pinecrest Studio",
            "Austin, TX · Hybrid",
            "About the job",
            "Easy Apply",
            "☆ Save",
        ] {
            assert!(s.contains(needle), "missing {needle:?} in:\n{s}");
        }
        press(&mut app, KeyCode::Char('e'));
        let s = screen(&app, 120, 30);
        assert!(s.contains("✔ Applied") && !s.contains("Easy Apply"), "{s}");
    }

    #[test]
    fn messaging_shows_thread_and_reply_box() {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        let s = screen(&app, 120, 30);
        assert!(s.contains("Select a conversation"), "{s}");
        assert!(s.contains("● Maren Holt"), "unread marker:\n{s}");

        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "Sounds good");
        let s = screen(&app, 120, 30);
        assert!(!s.contains("● Maren Holt"), "marked read:\n{s}");
        assert!(s.contains("pair on the caching write-up"), "{s}");
        assert!(s.contains("Sounds good▏"), "{s}");

        press(&mut app, KeyCode::Enter);
        let s = screen(&app, 120, 30);
        assert!(s.contains("Jordan Vale · now"), "{s}");
        assert!(s.contains("You: Sounds good"), "list preview:\n{s}");
    }

    #[test]
    fn every_hint_set_fits_80_columns() {
        let mut seen = Vec::new();
        for (name, mut app) in every_state() {
            app.status = None;
            let set = hints(&app);
            let width = hints_line(set).width();
            assert!(width <= 80, "{name}: {width} columns");
            let footer = screen(&app, 80, 24)
                .lines()
                .last()
                .expect("screen has rows")
                .trim_end()
                .to_owned();
            let (key, what) = set.last().expect("hint sets are not empty");
            assert!(
                footer.ends_with(&format!("{key} {what}")),
                "{name}: {footer:?}"
            );
            if !seen.contains(&set.as_ptr()) {
                seen.push(set.as_ptr());
            }
        }
        assert_eq!(seen.len(), 10, "every hint set was checked");
    }

    fn header_row(app: &App, width: u16, height: u16) -> String {
        screen(app, width, height)
            .lines()
            .nth(1)
            .expect("the header has 3 rows")
            .to_owned()
    }

    /// The tab numbers outside parentheses, and the badges inside them.
    fn tabs_and_badges(row: &str) -> (String, Vec<String>) {
        let mut numbers = String::new();
        let mut badges = Vec::new();
        let mut badge: Option<String> = None;
        for ch in row.chars() {
            match (ch, badge.as_mut()) {
                ('(', None) => badge = Some(String::new()),
                (')', Some(_)) => badges.extend(badge.take()),
                (_, Some(b)) => b.push(ch),
                (d, None) if d.is_ascii_digit() => numbers.push(d),
                _ => {}
            }
        }
        (numbers, badges)
    }

    #[test]
    fn narrow_header_keeps_logo_tabs_and_badges() {
        let mut app = App::new();
        for (width, height, tabs) in [
            (
                80,
                24,
                "1 Home  2 My Network (3)  3 Jobs  4 Messaging (2)  5 Notifications (3)",
            ),
            (
                70,
                30,
                "1 Home  2 Network (3)  3 Jobs  4 Msgs (2)  5 Notifs (3)",
            ),
            (60, 20, "1 Home  2 (3)  3  4 (2)  5 (3)"),
        ] {
            let row = header_row(&app, width, height);
            assert!(row.starts_with("│ in "), "{width}x{height}: logo: {row:?}");
            assert!(
                row.ends_with(&format!("{tabs}│")),
                "{width}x{height}: {row:?}"
            );
            assert!(!row.contains("Search"), "search goes first: {row:?}");
            let (numbers, badges) = tabs_and_badges(&row);
            assert_eq!(numbers, "12345", "{width}x{height}: {row:?}");
            assert_eq!(badges, ["3", "2", "3"], "{width}x{height}: {row:?}");
        }

        let row = header_row(&app, 120, 30);
        assert!(row.contains("⌕ Search"), "{row:?}");
        assert!(row.contains("5 Notifications (3)│"), "{row:?}");

        // With only numbers, the active tab keeps its name.
        press(&mut app, KeyCode::Char('3'));
        let row = header_row(&app, 60, 20);
        assert!(row.ends_with("1  2 (3)  3 Jobs  4 (2)  5 (3)│"), "{row:?}");
    }

    #[test]
    fn header_fits_down_to_29_columns() {
        for (name, app) in every_state() {
            let want: Vec<String> = Tab::ALL
                .iter()
                .map(|&t| app.badge(t))
                .filter(|&b| b > 0)
                .map(|b| b.to_string())
                .collect();
            for width in 29..=140 {
                let row = header_row(&app, width, 10);
                assert!(row.starts_with("│ in "), "{name} at {width}: {row:?}");
                let (numbers, badges) = tabs_and_badges(&row);
                assert_eq!(numbers, "12345", "{name} at {width}: {row:?}");
                assert_eq!(badges, want, "{name} at {width}: {row:?}");
            }
        }
    }

    /// Row `y` from column `x0` up to `x1`, reading a double-width cell once.
    fn text_between(buf: &Buffer, y: u16, x0: u16, x1: u16) -> String {
        let mut text = String::new();
        let mut x = x0;
        while x < x1 {
            let sym = buf[(x, y)].symbol();
            text.push_str(sym);
            x += u16_of(width_of(sym).max(1));
        }
        text
    }

    /// The text inside columns `x0..x1` of rows `ys`, each row trimmed, joined
    /// without separators so wrapped words join up again.
    fn text_in(buf: &Buffer, ys: std::ops::Range<u16>, x0: u16, x1: u16) -> String {
        ys.map(|y| text_between(buf, y, x0, x1).trim().to_owned())
            .collect()
    }

    const WIDE: &str = "a日本語のテキストを入力しています。とても長い文章です";

    #[test]
    fn wide_text_stays_inside_the_composer_and_card() {
        let (width, height) = (40, 16);
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, WIDE);

        let buf = buffer(&app, width, height);
        let popup = composer_area(Rect::new(0, 3, width, height - 4));
        assert!(
            width_of(WIDE) > usize::from(popup.width),
            "the draft must wrap"
        );
        for y in popup.top() + 1..popup.bottom() - 1 {
            for x in [popup.left(), popup.right() - 1] {
                assert_eq!(buf[(x, y)].symbol(), "│", "border at {x},{y}");
            }
        }
        let inside = text_in(
            &buf,
            popup.top() + 1..popup.bottom() - 1,
            popup.left() + 1,
            popup.right() - 1,
        );
        assert!(
            inside.contains(&format!("{WIDE}▏")),
            "lost text: {inside:?}"
        );

        // Published, it wraps inside its card. At 40 columns the feed is the
        // only column, so every body row ends in a border.
        press(&mut app, KeyCode::Enter);
        let buf = buffer(&app, width, height);
        for y in 3..height - 1 {
            let edge = buf[(width - 1, y)].symbol();
            assert!("│╮╯".contains(edge), "row {y} ends in {edge:?}");
        }
        let card = text_in(&buf, 6..height - 1, 1, width - 1);
        assert!(card.contains(WIDE), "lost text: {card:?}");
    }

    #[test]
    fn wide_text_stays_inside_the_reply_box_and_thread() {
        let (width, height) = (60, 20);
        let mut app = App::new();
        press(&mut app, KeyCode::Char('4'));
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, WIDE);

        let buf = buffer(&app, width, height);
        let row = (0..height)
            .map(|y| text_between(&buf, y, 0, width))
            .find(|l| l.contains('▏'))
            .expect("cursor shown");
        // List border, thread border, reply border, the reply, then the
        // reply's and the thread's right borders.
        let parts: Vec<&str> = row.split('│').collect();
        assert_eq!(parts.len(), 7, "{row:?}");
        let shown = parts[4]
            .trim()
            .strip_suffix('▏')
            .expect("cursor at the end");
        assert!(WIDE.ends_with(shown), "{shown:?} is the end of the reply");
        // One padding column on each side, and one for the cursor.
        let room = width_of(parts[4]) - 3;
        assert!(
            width_of(shown) + 1 >= room,
            "{shown:?} should fill {room} columns, less at most one"
        );

        // Sent, it wraps inside the thread.
        press(&mut app, KeyCode::Enter);
        let buf = buffer(&app, width, height);
        let list = MESSAGING_LIST_WIDTH.min(width / 2);
        for y in 4..height - 2 {
            assert_eq!(buf[(width - 1, y)].symbol(), "│", "row {y}");
        }
        let thread = text_in(&buf, 4..height - 1, list + 1, width - 1);
        assert!(thread.contains(WIDE), "lost text: {thread:?}");
    }

    #[test]
    fn notification_markers_last_one_visit() {
        let mut app = App::new();
        let unread: Vec<&str> = app
            .notifications
            .iter()
            .filter(|n| n.unread)
            .map(|n| n.text)
            .collect();
        press(&mut app, KeyCode::Char('5'));
        let s = screen(&app, 120, 30);
        assert!(!s.contains("Notifications ("), "badge cleared:\n{s}");
        assert_eq!(s.matches('●').count(), 3, "{s}");
        for text in &unread {
            assert!(
                s.contains(&format!("● {text}")),
                "{text:?} marked new:\n{s}"
            );
        }

        press(&mut app, KeyCode::Char('j'));
        assert_eq!(screen(&app, 120, 30).matches('●').count(), 3);

        press(&mut app, KeyCode::Char('1'));
        press(&mut app, KeyCode::Char('5'));
        let s = screen(&app, 120, 30);
        assert_eq!(
            s.matches('●').count(),
            0,
            "markers gone after leaving:\n{s}"
        );
        assert!(!s.contains("Notifications ("), "{s}");
    }

    fn assert_single_width(what: &str, s: &str) {
        for ch in s.chars().filter(|&c| c != '\n') {
            let w = Span::raw(ch.to_string()).width();
            assert_eq!(w, 1, "{what}: {ch:?} (U+{:04X}) in {s:?}", u32::from(ch));
        }
    }

    #[test]
    fn no_double_width_characters() {
        let app = App::new();
        let p = &app.profile;
        let mut strings = vec![p.name, p.headline];
        for post in &app.posts {
            strings.extend([post.author, post.headline, post.body.as_str()]);
            for c in &post.comments {
                strings.extend([c.author, c.headline, c.body.as_str()]);
            }
        }
        strings.extend(app.news.iter().map(|n| n.title));
        for person in app
            .invitations
            .iter()
            .chain(app.suggestions.iter().map(|s| &s.person))
        {
            strings.extend([person.name, person.headline]);
        }
        for job in &app.jobs {
            strings.extend([
                job.title,
                job.company,
                job.location,
                job.workplace.label(),
                job.description,
            ]);
        }
        for chat in &app.conversations {
            strings.extend([chat.name, chat.headline]);
            strings.extend(chat.messages.iter().map(|m| m.body.as_str()));
        }
        strings.extend(app.notifications.iter().map(|n| n.text));
        strings.extend(Tab::ALL.iter().map(|t| t.label()));
        for s in strings {
            assert_single_width("data", s);
        }

        // Every label the renderer draws, on every screen.
        for (name, app) in every_state() {
            let buf = buffer(&app, 140, 40);
            for (key, what) in hints(&app) {
                assert_single_width(name, key);
                assert_single_width(name, what);
            }
            for cell in buf.content() {
                let sym = cell.symbol();
                assert!(
                    Span::raw(sym).width() <= 1,
                    "{name}: {sym:?} is double width"
                );
            }
        }
    }

    /// Where the search overlay is on a `width`×`height` screen.
    fn search_popup(width: u16, height: u16) -> Rect {
        search_area(screen_areas(Rect::new(0, 0, width, height))[1])
    }

    /// Each row inside the search overlay's border, trimmed.
    fn search_rows(app: &App, width: u16, height: u16) -> Vec<String> {
        let buf = buffer(app, width, height);
        let popup = search_popup(width, height);
        (popup.top() + 1..popup.bottom() - 1)
            .map(|y| {
                text_between(&buf, y, popup.left() + 1, popup.right() - 1)
                    .trim()
                    .to_owned()
            })
            .collect()
    }

    /// The runs of bold cells inside `area`, row by row.
    fn bold_runs(buf: &Buffer, area: Rect) -> Vec<String> {
        let mut runs = Vec::new();
        for y in area.top()..area.bottom() {
            let mut run = String::new();
            let mut x = area.left();
            while x < area.right() {
                let cell = &buf[(x, y)];
                if cell.modifier.contains(Modifier::BOLD) {
                    run.push_str(cell.symbol());
                } else if !run.is_empty() {
                    runs.push(std::mem::take(&mut run));
                }
                x += u16_of(width_of(cell.symbol()).max(1));
            }
            if !run.is_empty() {
                runs.push(run);
            }
        }
        runs
    }

    fn searching(text: &str) -> App {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, text);
        app
    }

    #[test]
    fn search_overlay_groups_results_under_headings() {
        let app = searching("bright");
        let rows = search_rows(&app, 120, 30);
        assert!(rows[0].starts_with("⌕ bright▏ "), "{rows:#?}");
        assert!(rows[0].ends_with(" 5 results"), "{rows:#?}");
        let at = |want: &str| {
            rows.iter()
                .position(|r| r == want)
                .unwrap_or_else(|| panic!("no row {want:?} in {rows:#?}"))
        };
        let (people, jobs, posts) = (at("People"), at("Jobs"), at("Posts"));
        assert!(people < jobs && jobs < posts, "{rows:#?}");
        // Two lines per result, the selected one with the bar.
        for (offset, want) in [
            (1, "▌Baptiste Moreau"),
            (2, "▌Frontend Engineer at Brightkettle · Invitation"),
            (3, "Mateo Achebe"),
            (4, "Developer Advocate at Brightkettle · Connect"),
        ] {
            assert_eq!(rows[people + offset], want, "{rows:#?}");
        }
        assert_eq!(rows[jobs + 1], "Developer Advocate");
        assert_eq!(rows[jobs + 2], "Brightkettle · Anywhere in Europe · Remote");
        assert_eq!(
            rows[posts + 1],
            "Tobias Ferreira-Lund · Product Manager at Brightkettle"
        );
        // The body on one line, newline as a space, cut with `…`.
        let body = &rows[posts + 2];
        assert!(
            body.starts_with(
                "Unpopular opinion: the best meeting is the one that turns into a doc. Wri"
            ),
            "{body:?}"
        );
        assert!(body.ends_with('…'), "{body:?}");
        assert_eq!(
            rows[posts + 3],
            "Hana Sorensen · Data Scientist at Brightkettle"
        );

        // The header's search box shows the query.
        let header = header_row(&app, 120, 30);
        assert!(header.contains("⌕ bright▏"), "{header:?}");
        assert!(!header.contains("Search"), "{header:?}");

        // Groups with no results have no heading.
        let rows = search_rows(&searching("baptiste"), 120, 30);
        assert!(rows.contains(&"People".to_owned()), "{rows:#?}");
        assert!(!rows.contains(&"Jobs".to_owned()) && !rows.contains(&"Posts".to_owned()));

        let rows = search_rows(&searching(""), 120, 30);
        assert_eq!(rows[2], SEARCH_PROMPT, "{rows:#?}");
        let rows = search_rows(&searching("zqzq"), 120, 30);
        assert_eq!(rows[2], NO_RESULTS, "{rows:#?}");
        assert!(!rows.contains(&"People".to_owned()));
        // Only spaces is no query at all.
        let rows = search_rows(&searching("   "), 120, 30);
        assert_eq!(rows[2], SEARCH_PROMPT, "{rows:#?}");
    }

    #[test]
    fn search_overlay_scrolls_to_the_selection_and_follows_state() {
        // "a" matches more results than fit: the last stays visible.
        let mut app = searching("a");
        let total = app.search_hits("a").len();
        for _ in 0..total {
            press(&mut app, KeyCode::Down);
        }
        let rows = search_rows(&app, 80, 24);
        let last = rows.iter().rposition(|r| r.starts_with('▌'));
        assert!(last.is_some(), "selection visible: {rows:#?}");
        assert!(!rows.contains(&"People".to_owned()), "scrolled: {rows:#?}");

        // A suggestion you've connected to shows Pending.
        let mut app = App::new();
        press(&mut app, KeyCode::Char('2'));
        for _ in 0..6 {
            press(&mut app, KeyCode::Char('j'));
        }
        press(&mut app, KeyCode::Char('c'));
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "mateo");
        let rows = search_rows(&app, 120, 30);
        assert!(
            rows.contains(&"▌Developer Advocate at Brightkettle · Pending".to_owned()),
            "{rows:#?}"
        );
    }

    #[test]
    fn match_ranges_map_back_to_the_original_text() {
        let ranges = |text: &str, needle: &str| -> Vec<(usize, usize)> {
            match_ranges(text, needle)
                .into_iter()
                .map(|r| (r.start, r.end))
                .collect()
        };
        assert_eq!(ranges("Brightkettle", "bright"), [(0, 6)]);
        assert_eq!(ranges("aaa", "aa"), [(0, 2), (1, 3)], "overlapping");
        assert_eq!(ranges("abc", ""), []);
        // `İ` (2 bytes) lowercases to `i` plus a combining dot (3 bytes).
        assert_eq!(ranges("İstanbul", "stan"), [(2, 6)]);
        assert_eq!(ranges("xİstanbul", "i̇stan"), [(1, 7)]);
        // A match that covers part of a character covers all of it.
        assert_eq!(ranges("İstanbul", "i"), [(0, 2)]);
        // A final sigma lowercases to `ς`, as in `App::search_hits`.
        assert_eq!(ranges("ΟΔΟΣ", "οδος"), [(0, 8)]);
        assert_eq!(ranges("会議は日本語で", "日本語"), [(9, 18)]);

        let style = Style::new();
        let bold = |spans: &[Span]| -> Vec<String> {
            spans
                .iter()
                .filter(|s| s.style.add_modifier.contains(Modifier::BOLD))
                .map(|s| s.content.to_string())
                .collect()
        };
        let line = hit_line(&[("a Bright b", style, true)], "bright", 20);
        assert_eq!(bold(&line), ["Bright"]);
        // Only the part of the line that fits counts.
        let line = hit_line(&[("abc bright", style, true)], "bright", 8);
        assert!(bold(&line).is_empty(), "{line:?}");
        let line = hit_line(&[("abc bright bright", style, true)], "bright", 8);
        assert!(bold(&line).is_empty(), "{line:?}");
        // Unsearched parts never highlight.
        let line = hit_line(
            &[("bright", style, false), (" x bright", style, true)],
            "bright",
            20,
        );
        assert_eq!(bold(&line), ["bright"]);
        assert_eq!(line[0].content, "bright");
        assert!(!line[0].style.add_modifier.contains(Modifier::BOLD));
        assert!(hit_line(&[("abc", style, true)], "b", 0).is_empty());
    }

    #[test]
    fn search_highlights_exactly_the_match() {
        let publish = |text: &str| {
            let mut app = App::new();
            press(&mut app, KeyCode::Char('p'));
            type_str(&mut app, text);
            press(&mut app, KeyCode::Enter);
            press(&mut app, KeyCode::Char('/'));
            app
        };

        let mut app = publish("İstanbul office opening");
        type_str(&mut app, "stan");
        assert_eq!(app.search_hits("stan"), [SearchHit::Post(0)]);
        for (width, height) in [(120, 30), (60, 20), (30, 12), (12, 8)] {
            let buf = buffer(&app, width, height);
            let popup = search_popup(width, height);
            let runs = bold_runs(&buf, popup.inner(ratatui::layout::Margin::new(1, 1)));
            if width >= 30 {
                assert_eq!(runs, ["stan"], "{width}x{height}");
            } else {
                assert!(runs.is_empty(), "the match doesn't fit: {runs:?}");
            }
        }
        let rows = search_rows(&app, 120, 30);
        assert!(
            rows.contains(&"▌İstanbul office opening".to_owned()),
            "{rows:#?}"
        );

        let mut app = publish("会議は日本語で行います");
        type_str(&mut app, "日本語");
        let buf = buffer(&app, 120, 30);
        let popup = search_popup(120, 30);
        let runs = bold_runs(&buf, popup.inner(ratatui::layout::Margin::new(1, 1)));
        assert_eq!(runs, ["日本語"]);

        // Every bold run in a busy overlay is a match.
        let app = searching("BRIGHT");
        let buf = buffer(&app, 120, 30);
        let popup = search_popup(120, 30);
        let runs = bold_runs(&buf, popup.inner(ratatui::layout::Margin::new(1, 1)));
        assert_eq!(runs, ["Bright"; 5]);
    }

    #[test]
    fn search_overlay_stays_inside_its_border_at_every_size() {
        let long: String = "日本語 search".repeat(300);
        assert_eq!(long.chars().count(), 3_000);
        let states = [
            ("empty", searching("")),
            ("matches", searching("bright")),
            ("long", searching(&long)),
            ("no match", searching("zqzq")),
        ];
        std::thread::scope(|scope| {
            for (name, app) in &states {
                scope.spawn(move || {
                    let mut terminal =
                        Terminal::new(TestBackend::new(1, 1)).expect("test backend is infallible");
                    for width in 1..=140 {
                        for height in 1..=40 {
                            terminal.backend_mut().resize(width, height);
                            terminal
                                .draw(|f| draw(f, app))
                                .unwrap_or_else(|e| panic!("{name} at {width}x{height}: {e}"));
                            let popup = search_popup(width, height);
                            if popup.width < 2 || popup.height < 2 {
                                continue;
                            }
                            let buf = terminal.backend().buffer();
                            for y in popup.top()..popup.bottom() {
                                let row = text_between(buf, y, popup.left(), popup.right());
                                let ends = match y {
                                    y if y == popup.top() => ('╭', '╮'),
                                    y if y == popup.bottom() - 1 => ('╰', '╯'),
                                    _ => ('│', '│'),
                                };
                                assert!(
                                    row.starts_with(ends.0)
                                        && row.ends_with(ends.1)
                                        && width_of(&row) == usize_of(popup.width),
                                    "{name} at {width}x{height}, row {y}: {row:?}"
                                );
                            }
                        }
                    }
                });
            }
        });

        // The input row and the header box show the end of a long query.
        let app = &states[2].1;
        let rows = search_rows(app, 120, 30);
        let shown = rows[0]
            .strip_prefix("⌕ ")
            .and_then(|r| r.strip_suffix('▏'))
            .expect("icon and cursor");
        assert!(width_of(shown) > 60, "{shown:?}");
        assert!(long.ends_with(shown), "{shown:?}");
        let header = header_row(app, 120, 30);
        assert!(header.contains("search▏"), "{header:?}");
        // The header keeps the logo, every tab and every badge.
        for width in [140, 120, 80, 70, 60] {
            let row = header_row(app, width, 30);
            assert!(row.starts_with("│ in "), "{width}: {row:?}");
            let (numbers, badges) = tabs_and_badges(&row);
            assert_eq!(numbers, "12345", "{width}: {row:?}");
            assert_eq!(badges, ["3", "2", "3"], "{width}: {row:?}");
        }
    }

    #[test]
    fn search_labels_are_single_width() {
        for label in [
            SEARCH_PROMPT,
            NO_RESULTS,
            " Search ",
            "People",
            "Jobs",
            "Posts",
            " · Invitation",
            " · Connect",
            " · Pending",
            "⌕ ",
            "▏",
            "…",
            "─",
            "▌",
        ] {
            assert_single_width("search", label);
        }
        let app = searching("");
        for (key, what) in hints(&app) {
            assert_single_width("search hints", key);
            assert_single_width("search hints", what);
        }
        assert!(hints_line(hints(&app)).width() <= 80);
    }

    /// The text of `spans`, and its bold pieces.
    fn spans_text(spans: &[Span]) -> (String, Vec<String>) {
        let text = spans.iter().map(|s| s.content.as_ref()).collect();
        let bold = spans
            .iter()
            .filter(|s| s.style.add_modifier.contains(Modifier::BOLD))
            .map(|s| s.content.to_string())
            .collect();
        (text, bold)
    }

    /// Whether `excerpt` appears in `body` starting on a word boundary.
    fn starts_a_word(body: &str, excerpt: &str) -> bool {
        body.match_indices(excerpt)
            .any(|(at, _)| at == 0 || body[..at].ends_with(' '))
    }

    fn publish_and_search(body: &str, query: &str) -> App {
        let mut app = App::new();
        press(&mut app, KeyCode::Char('p'));
        type_str(&mut app, body);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, query);
        app
    }

    /// The overlay's bold runs and the text of the row holding the selected
    /// post's body line, at `width`×`height`.
    fn post_body_row(app: &App, width: u16, height: u16) -> (Vec<String>, String) {
        let buf = buffer(app, width, height);
        let popup = search_popup(width, height);
        let runs = bold_runs(&buf, popup.inner(ratatui::layout::Margin::new(1, 1)));
        let rows = search_rows(app, width, height);
        let at = rows
            .iter()
            .position(|r| r == "Posts")
            .unwrap_or_else(|| panic!("no posts in {rows:#?}"));
        (runs, rows[at + 2].clone())
    }

    #[test]
    fn excerpt_starts_on_a_word_unless_the_word_is_too_wide() {
        let text = "alpha beta gamma";
        assert_eq!(excerpt_start(text, 11, 6), 6, "\"beta \" fits");
        assert_eq!(
            excerpt_start(text, 11, 20),
            0,
            "the earliest word that fits"
        );
        assert_eq!(excerpt_start(text, 11, 4), 11, "\"ta \" would cut a word");
        assert_eq!(excerpt_start(text, 11, 0), 11);
        // A match inside a word wider than the budget cuts it.
        assert_eq!(excerpt_start("alpha betagamma", 10, 3), 7);
        assert_eq!(excerpt_start("alpha betagamma", 10, 4), 6);
        // CJK has no spaces, so it is cut by display width.
        assert_eq!(excerpt_start("日本語のテキスト", 9, 4), 3);
        assert_eq!(excerpt_start("日本語のテキスト", 9, 5), 3);
        // Never a leading space.
        assert_eq!(excerpt_start("ab  cd", 4, 1), 4);
    }

    #[test]
    fn body_line_shows_an_excerpt_around_a_hidden_match() {
        let style = Style::new();
        // Visible from the start: exactly the plain line.
        let body = "I'm happy to share that our team cut the CI queue";
        assert_eq!(
            body_line(body, "happy", style, 30),
            hit_line(&[(body, style, true)], "happy", 30)
        );
        assert_eq!(
            spans_text(&body_line(body, "happy", style, 30)).1,
            ["happy"]
        );
        assert_eq!(
            body_line(body, "zzz", style, 30),
            hit_line(&[(body, style, true)], "zzz", 30)
        );

        // Hidden: `…`, whole words, the match, `…`.
        let body = "I'm happy to share that our team cut the CI queue from 40 minutes to 6.";
        let (text, bold) = spans_text(&body_line(body, "cut", style, 24));
        assert_eq!(bold, ["cut"]);
        assert!(text.starts_with('…') && text.ends_with('…'), "{text:?}");
        assert_eq!(width_of(&text), 24, "{text:?}");
        let excerpt = text.trim_matches('…');
        assert!(starts_a_word(body, excerpt), "{excerpt:?}");
        assert!(!excerpt.starts_with("cut"), "context first: {excerpt:?}");

        // `İ` changes length when lowercased; the excerpt keeps it whole.
        let body = "a b c d e f g h i j k l m n o p İstanbul office opening";
        let (text, bold) = spans_text(&body_line(body, "stan", style, 20));
        assert_eq!(bold, ["stan"], "{text:?}");
        assert!(text.contains("İstanbul"), "{text:?}");
        assert!(starts_a_word(body, text.trim_matches('…')), "{text:?}");
        assert!(width_of(&text) <= 20, "{text:?}");

        // Every width, for narrow-screen safety.
        for width in 0..40 {
            let (text, _) = spans_text(&body_line(body, "stan", style, width));
            assert!(width_of(&text) <= width, "{width}: {text:?}");
        }
    }

    #[test]
    fn skipped_shows_an_excerpt_with_the_match_bold() {
        let app = searching("skipped");
        assert_eq!(app.search_hits("skipped"), [SearchHit::Post(0)]);
        let (runs, row) = post_body_row(&app, 120, 30);
        assert_eq!(runs, ["skipped"], "{row:?}");
        let excerpt = row
            .strip_prefix("▌…")
            .and_then(|r| r.strip_suffix('…'))
            .unwrap_or_else(|| panic!("an excerpt: {row:?}"));
        let body = app.posts[0].body.replace('\n', " ");
        assert!(starts_a_word(&body, excerpt), "{excerpt:?}");
        assert!(
            excerpt.find("skipped").is_some_and(|i| i > 0),
            "{excerpt:?}"
        );
        // The plain start of the body is gone.
        assert!(!row.contains("I'm happy"), "{row:?}");
    }

    #[test]
    fn excerpt_for_a_match_at_the_end_of_a_long_body() {
        let body = format!(
            "Notes from the offsite. {}and last of all zebracorn.",
            "We talked about plans and more plans. ".repeat(6)
        );
        let app = publish_and_search(&body, "zebracorn");
        for (width, height) in [(120, 30), (60, 20)] {
            let (runs, row) = post_body_row(&app, width, height);
            assert_eq!(runs, ["zebracorn"], "{width}: {row:?}");
            let excerpt = row.strip_prefix("▌…").expect("an excerpt");
            assert!(
                excerpt.ends_with("zebracorn."),
                "no cut at the end: {row:?}"
            );
            assert!(body.ends_with(excerpt), "{excerpt:?}");
            assert!(starts_a_word(&body, excerpt), "{excerpt:?}");
            // With the whole end in view, the context fills the rest, short
            // of at most one word ("more " here) so as not to cut it. The
            // card has a border and a padding column on each side.
            let room = usize_of(search_popup(width, height).width) - 4;
            let shown = width_of(&row);
            assert!(shown <= room && shown + 6 > room, "{room}: {row:?}");
        }
    }

    #[test]
    fn excerpt_in_a_wide_character_body() {
        // The rest of the body fits, by display width, and leaves more than
        // the context's third of the row: no `…` at the end, and the context
        // fills the line. At 40 columns it would leave less, so the context
        // keeps its third and the end is cut.
        let body = format!("{}東京駅で会いましょう。", "会議の議事録です。".repeat(6));
        let app = publish_and_search(&body, "東京駅");
        for (width, height) in [(120, 30), (60, 20), (40, 16)] {
            let (runs, row) = post_body_row(&app, width, height);
            assert_eq!(runs, ["東京駅"], "{width}: {row:?}");
            let excerpt = row.strip_prefix("▌…").expect("an excerpt");
            let room = usize_of(search_popup(width, height).width) - 4;
            if width > 40 {
                assert!(body.ends_with(excerpt), "{width}: the end shows: {row:?}");
            } else {
                let excerpt = excerpt.strip_suffix('…').expect("cut at the end");
                assert!(body.contains(excerpt), "{excerpt:?}");
                assert!(!excerpt.starts_with("東京駅"), "context first: {excerpt:?}");
            }
            // Two columns per character, so at most one short.
            assert!(
                width_of(&row) <= room && width_of(&row) + 2 > room,
                "{room}: {row:?}"
            );
        }

        // A long rest: cut with `…` at the end, the match still whole.
        let body = format!(
            "{}東京駅{}",
            "会議の議事録です。".repeat(6),
            "で会いましょう。".repeat(6)
        );
        let app = publish_and_search(&body, "東京駅");
        for (width, height) in [(120, 30), (60, 20), (40, 16)] {
            let (runs, row) = post_body_row(&app, width, height);
            assert_eq!(runs, ["東京駅"], "{width}: {row:?}");
            let excerpt = row
                .strip_prefix("▌…")
                .and_then(|r| r.strip_suffix('…'))
                .unwrap_or_else(|| panic!("{width}: an excerpt cut at both ends: {row:?}"));
            assert!(body.contains(excerpt), "{excerpt:?}");
            assert!(!excerpt.starts_with("東京駅"), "context first: {excerpt:?}");
            let room = usize_of(search_popup(width, height).width) - 4;
            assert!(width_of(&row) <= room, "{room}: {row:?}");
        }
    }

    #[test]
    fn a_visible_body_match_keeps_the_plain_line() {
        let app = searching("CI queue");
        let (runs, row) = post_body_row(&app, 120, 30);
        assert_eq!(runs, ["CI queue"]);
        assert!(row.starts_with("▌I'm happy to share"), "{row:?}");
    }

    #[test]
    fn search_input_shows_the_result_count() {
        let first = |query: &str| search_rows(&searching(query), 120, 30)[0].clone();
        let row = first("bright");
        assert!(
            row.starts_with("⌕ bright▏") && row.ends_with(" 5 results"),
            "{row:?}"
        );
        let row = first("lisbon");
        assert!(
            row.starts_with("⌕ lisbon▏") && row.ends_with(" 1 result"),
            "{row:?}"
        );
        assert_eq!(first(""), "⌕ ▏", "nothing for an empty query");
        assert_eq!(first("zqzq"), "⌕ zqzq▏", "nothing extra for no results");

        // A long query keeps the count at full width, and its own end.
        let query = "Unpopular opinion: the best meeting is the one that turns into a doc";
        let row = first(query);
        assert!(
            row.contains("into a doc▏") && row.ends_with(" 1 result"),
            "{row:?}"
        );
    }

    #[test]
    fn the_count_goes_before_the_end_of_the_query() {
        let text = |query: &str, n: usize, width: usize| -> String {
            search_input(query, n, width)
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect()
        };
        // A short query: whole until the count has gone.
        let mut dropped = false;
        for width in (3..=40).rev() {
            let row = text("bright", 5, width);
            if row.contains("results") {
                assert!(!dropped, "the count came back at {width}");
                assert!(row.starts_with("⌕ bright▏"), "{width}: {row:?}");
                assert_eq!(width_of(&row), width, "{width}: {row:?}");
            } else {
                dropped = true;
                if width >= 9 {
                    assert_eq!(row, "⌕ bright▏", "{width}: the query stays");
                }
            }
        }
        assert!(text("bright", 5, 20).ends_with("5 results"));
        assert_eq!(text("bright", 5, 19), "⌕ bright▏");

        // A long query: the count shows while the end keeps 10 columns.
        let query = "日本語 search ".repeat(20);
        for width in 4..=80 {
            let row = text(&query, 3, width);
            let shown = row
                .strip_prefix("⌕ ")
                .and_then(|r| r.split('▏').next())
                .expect("icon and cursor");
            assert!(query.ends_with(shown), "{width}: {row:?}");
            if row.contains("results") {
                assert!(width_of(shown) >= QUERY_MIN_WIDTH - 1, "{width}: {row:?}");
            } else {
                assert!(
                    width_of(shown) >= width - 4,
                    "{width}: the end fills the row"
                );
            }
        }
        assert!(text(&query, 3, 25).ends_with("3 results"));
        assert!(!text(&query, 3, 23).contains("results"));
    }
}
