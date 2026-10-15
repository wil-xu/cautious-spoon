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
use crate::app::Tab;
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

pub(crate) fn draw(frame: &mut Frame, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

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
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ⌕ ", Style::new().fg(DIM)),
                Span::styled("Search", Style::new().fg(DIM)),
            ]))
            .bg(HIGHLIGHT_BG),
            search,
        );
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
        (_, Mode::PostDetail { .. }) => &[
            ("j/k", "comments"),
            ("l", "like"),
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

fn render_home(frame: &mut Frame, app: &App, area: Rect) {
    let show_profile = area.width >= PROFILE_MIN_WIDTH;
    let show_sidebar = area.width >= SIDEBAR_MIN_WIDTH;
    let [left, center, right] = Layout::horizontal([
        Constraint::Length(if show_profile { PROFILE_WIDTH } else { 0 }),
        Constraint::Min(0),
        Constraint::Length(if show_sidebar { SIDEBAR_WIDTH } else { 0 }),
    ])
    .areas(area);

    if show_profile {
        render_profile(frame, app, left);
    }
    if show_sidebar {
        render_news(frame, app, right);
    }
    match app.mode {
        Mode::PostDetail { comment } => render_post_detail(frame, app, center, comment),
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

fn render_post_detail(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let Some(post) = app.selected_post() else {
        return;
    };
    let block = card(" Post ", true);
    let inner = block.inner(area);
    let width = usize_of(inner.width);
    frame.render_widget(block, area);

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
            wrap(comment.body, text_width)
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
    let scroll = selected_end.saturating_sub(usize_of(inner.height));
    frame.render_widget(Paragraph::new(lines).scroll((u16_of(scroll), 0)), inner);
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
        vec![
            ("home", at(&[], "")),
            ("detail", at(&[KeyCode::Char('j'), KeyCode::Enter], "")),
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
        ]
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
        assert_eq!(seen.len(), 8, "every hint set was checked");
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
                strings.extend([c.author, c.headline, c.body]);
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
}
