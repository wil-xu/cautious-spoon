//! Seed content. Every person, company, post and job here is fictional.

#[derive(Debug, Clone)]
pub(crate) struct Profile {
    pub name: &'static str,
    pub headline: &'static str,
    pub connections: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Comment {
    pub author: &'static str,
    pub headline: &'static str,
    pub hours_ago: u32,
    /// Owned, because comments you post hold typed text.
    pub body: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Post {
    pub author: &'static str,
    pub headline: &'static str,
    pub hours_ago: u32,
    /// Owned, because posts published from the composer hold typed text.
    pub body: String,
    pub reactions: u32,
    pub comments: Vec<Comment>,
    pub liked: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct News {
    pub title: &'static str,
    pub hours_ago: u32,
    pub readers: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Person {
    pub name: &'static str,
    pub headline: &'static str,
    pub mutual: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Suggestion {
    pub person: Person,
    /// `true` once you've sent an invitation ("Pending"), `false` for "Connect".
    pub pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Workplace {
    Remote,
    Hybrid,
    OnSite,
}

impl Workplace {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Workplace::Remote => "Remote",
            Workplace::Hybrid => "Hybrid",
            Workplace::OnSite => "On-site",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Job {
    pub title: &'static str,
    pub company: &'static str,
    pub location: &'static str,
    pub workplace: Workplace,
    pub days_ago: u32,
    pub applicants: u32,
    pub description: &'static str,
    pub saved: bool,
    pub applied: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Message {
    pub from_me: bool,
    /// Owned, because replies hold typed text.
    pub body: String,
    pub hours_ago: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Conversation {
    pub name: &'static str,
    pub headline: &'static str,
    pub messages: Vec<Message>,
    pub unread: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Notification {
    pub text: &'static str,
    pub hours_ago: u32,
    pub unread: bool,
    /// Unread when you last opened the Notifications tab. Drawn with `●`
    /// until you leave the tab, even though `unread` is already cleared.
    pub is_new: bool,
}

pub(crate) fn profile() -> Profile {
    Profile {
        name: "Jordan Vale",
        headline: "Software Engineer at Quillfeather Systems",
        connections: 312,
    }
}

fn c(author: &'static str, headline: &'static str, hours_ago: u32, body: &'static str) -> Comment {
    Comment {
        author,
        headline,
        hours_ago,
        body: body.to_owned(),
    }
}

fn post(
    author: &'static str,
    headline: &'static str,
    hours_ago: u32,
    reactions: u32,
    body: &'static str,
    comments: Vec<Comment>,
) -> Post {
    Post {
        author,
        headline,
        hours_ago,
        body: body.to_owned(),
        reactions,
        comments,
        liked: false,
    }
}

pub(crate) fn posts() -> Vec<Post> {
    vec![
        post(
            "Maren Holt",
            "Staff Engineer at Quillfeather Systems",
            2,
            342,
            "I'm happy to share that our team cut the CI queue from 40 minutes to 6.\n\
             No heroics: we cached the dependency build, split the slowest test \
             target in four, and deleted a suite that had been skipped since \
             before most of us joined. Small boring fixes add up.",
            vec![
                c(
                    "Felix Abernathy",
                    "Software Engineering Intern at Quillfeather Systems",
                    1,
                    "Can confirm. I pushed twice before lunch today and both runs finished.",
                ),
                c(
                    "Ravi Tenorio",
                    "Engineering Manager at Quillfeather Systems",
                    1,
                    "The deleted suite is my favorite part. Nobody missed it.",
                ),
                c(
                    "Hana Sorensen",
                    "Data Scientist at Brightkettle",
                    0,
                    "Would love a write-up of the caching setup.",
                ),
            ],
        ),
        post(
            "Tobias Ferreira-Lund",
            "Product Manager at Brightkettle",
            4,
            1_204,
            "Unpopular opinion: the best meeting is the one that turns into a doc.\n\
             Write the decision down, share it, and give people a day to comment. \
             You get better feedback from the quiet half of the room, and nobody \
             has to find a free slot across four time zones.",
            vec![
                c(
                    "Casey Lindqvist",
                    "Design Lead at Pinecrest Studio",
                    3,
                    "Agree, as long as the doc has a deadline. Otherwise it is a meeting that never ends.",
                ),
                c(
                    "Ines Okonkwo",
                    "Technical Recruiter at Harborline Talent",
                    2,
                    "Saving this for my next calendar cleanup.",
                ),
            ],
        ),
        post(
            "Ines Okonkwo",
            "Technical Recruiter at Harborline Talent",
            6,
            87,
            "We're hiring! Harborline Logistics is looking for a Site Reliability \
             Engineer to keep our shipping dashboards green. On-site in Rotterdam, \
             great team, and a canal view from the oncall desk. Check the Jobs tab \
             or message me directly.",
            vec![c(
                "Noor Haddad",
                "Site Reliability Engineer at Quillfeather Systems",
                5,
                "The canal view is a strong selling point for oncall.",
            )],
        ),
        post(
            "Casey Lindqvist",
            "Design Lead at Pinecrest Studio",
            9,
            518,
            "Five years at Pinecrest Studio today. Things I've learned:\n\
             1. Sketch on paper first.\n\
             2. Every empty state deserves a friendly sentence.\n\
             3. Ask the support team what confuses people. They always know.\n\
             4. Contrast ratios are not optional.\n\
             5. Bring snacks to design reviews.",
            vec![
                c(
                    "Tobias Ferreira-Lund",
                    "Product Manager at Brightkettle",
                    8,
                    "Number 5 is the real secret. Congratulations, Casey!",
                ),
                c(
                    "Maren Holt",
                    "Staff Engineer at Quillfeather Systems",
                    7,
                    "Happy work anniversary!",
                ),
            ],
        ),
        post(
            "Ravi Tenorio",
            "Engineering Manager at Quillfeather Systems",
            26,
            761,
            "A year into managing, the most useful thing I do is a weekly \
             fifteen-minute check-in with each person, with an agenda they own. \
             Second most useful: saying \"I don't know, let me find out\" and \
             then actually finding out.",
            vec![c(
                "Felix Abernathy",
                "Software Engineering Intern at Quillfeather Systems",
                20,
                "The agenda doc is great. I keep a running list of questions in mine.",
            )],
        ),
        post(
            "Hana Sorensen",
            "Data Scientist at Brightkettle",
            30,
            2_390,
            "Our churn model hit 99% accuracy on the first try. Exciting, until we \
             noticed it predicted \"stays\" for every single customer. Only 1% of \
             customers churn.\n\
             Always check the baseline before you celebrate.",
            vec![
                c(
                    "Wendell Achterberg",
                    "Founder at Tinderbox Robotics",
                    28,
                    "Classic. Precision and recall would like a word.",
                ),
                c(
                    "Ravi Tenorio",
                    "Engineering Manager at Quillfeather Systems",
                    27,
                    "Printing this and putting it on the team wall.",
                ),
            ],
        ),
        post(
            "Felix Abernathy",
            "Software Engineering Intern at Quillfeather Systems",
            50,
            1_032,
            "Today I merged my first pull request! It fixes a typo in an error \
             message that has said \"recieved\" for three years. Thank you to my \
             reviewer for leaving seven kind comments about a one-line change.",
            vec![c(
                "Maren Holt",
                "Staff Engineer at Quillfeather Systems",
                49,
                "Welcome aboard. The typo had it coming.",
            )],
        ),
        post(
            "Lucia Brandt",
            "Workplace Experience Manager at Quillfeather Systems",
            74,
            455,
            "Good news for floor three: the espresso machine has been repaired. \
             Please run the cleaning cycle after the last cup of the day, and \
             remember the oat milk lives on the bottom shelf now.",
            Vec::new(),
        ),
        post(
            "Wendell Achterberg",
            "Founder at Tinderbox Robotics",
            122,
            3_118,
            "Thrilled to announce that our warehouse robot can now stack mugs \
             without dropping them. It took 14 months, 2 redesigns of the gripper \
             and roughly 600 mugs. Huge thanks to the team, and to the local \
             thrift shops for the steady supply.",
            vec![
                c(
                    "Hana Sorensen",
                    "Data Scientist at Brightkettle",
                    100,
                    "600 mugs is a dataset. Respect.",
                ),
                c(
                    "Casey Lindqvist",
                    "Design Lead at Pinecrest Studio",
                    98,
                    "Can it stack the ones with the weird handles?",
                ),
                c(
                    "Wendell Achterberg",
                    "Founder at Tinderbox Robotics",
                    97,
                    "Next quarter's roadmap.",
                ),
            ],
        ),
    ]
}

pub(crate) fn news() -> Vec<News> {
    vec![
        News {
            title: "Tabs-versus-spaces truce enters its third week",
            hours_ago: 2,
            readers: 4_812,
        },
        News {
            title: "Offices report record demand for standing desks",
            hours_ago: 5,
            readers: 2_960,
        },
        News {
            title: "Study: stand-ups shorter when chairs are removed",
            hours_ago: 9,
            readers: 1_745,
        },
        News {
            title: "Terminal apps make a quiet comeback",
            hours_ago: 20,
            readers: 1_203,
        },
        News {
            title: "Four-day sprints win fans among small teams",
            hours_ago: 30,
            readers: 986,
        },
    ]
}

fn person(name: &'static str, headline: &'static str, mutual: u32) -> Person {
    Person {
        name,
        headline,
        mutual,
    }
}

pub(crate) fn invitations() -> Vec<Person> {
    vec![
        person(
            "Noor Haddad",
            "Site Reliability Engineer at Quillfeather Systems",
            14,
        ),
        person("Baptiste Moreau", "Frontend Engineer at Brightkettle", 3),
        person("Yuki Tanabe", "Technical Writer at Harborline Logistics", 7),
    ]
}

pub(crate) fn suggestions() -> Vec<Suggestion> {
    [
        person("Amara Lindgren", "Security Engineer at Larkspur Labs", 9),
        person(
            "Theo Vasquez-Hart",
            "Product Designer at Pinecrest Studio",
            5,
        ),
        person(
            "Saoirse Kimura",
            "Robotics Engineer at Tinderbox Robotics",
            2,
        ),
        person("Mateo Achebe", "Developer Advocate at Brightkettle", 11),
    ]
    .into_iter()
    .map(|person| Suggestion {
        person,
        pending: false,
    })
    .collect()
}

pub(crate) fn jobs() -> Vec<Job> {
    vec![
        Job {
            title: "Senior Rust Engineer",
            company: "Quillfeather Systems",
            location: "Lisbon, Portugal",
            workplace: Workplace::Remote,
            days_ago: 1,
            applicants: 48,
            description: "Join the platform team building the services behind \
                          Quillfeather's document editor. You'll own a Rust \
                          storage layer, review designs, and mentor two junior \
                          engineers.\n\
                          You have shipped Rust in production, enjoy writing \
                          tests, and can explain a tricky bug to a non-engineer.",
            saved: true,
            applied: false,
        },
        Job {
            title: "Terminal UI Designer",
            company: "Pinecrest Studio",
            location: "Austin, TX",
            workplace: Workplace::Hybrid,
            days_ago: 3,
            applicants: 17,
            description: "Design keyboard-first interfaces for developer tools \
                          that live in the terminal. Box-drawing characters, \
                          color palettes that work on dark and light themes, and \
                          layouts that survive an 80-column window.\n\
                          Three days a week in the Austin studio.",
            saved: false,
            applied: false,
        },
        Job {
            title: "Site Reliability Engineer",
            company: "Harborline Logistics",
            location: "Rotterdam, Netherlands",
            workplace: Workplace::OnSite,
            days_ago: 6,
            applicants: 112,
            description: "Keep the shipment tracking platform fast and boring. \
                          You'll run the oncall rotation, tune alerts until they \
                          mean something, and automate the runbooks that still \
                          live in a wiki.\n\
                          The oncall desk has a canal view.",
            saved: false,
            applied: false,
        },
        Job {
            title: "Developer Advocate",
            company: "Brightkettle",
            location: "Anywhere in Europe",
            workplace: Workplace::Remote,
            days_ago: 2,
            applicants: 203,
            description: "Help developers succeed with Brightkettle's analytics \
                          APIs. Write tutorials, build sample apps, speak at \
                          meetups, and bring what you hear back to the product \
                          team.\n\
                          Some travel, about one week per quarter.",
            saved: false,
            applied: false,
        },
        Job {
            title: "Data Engineer",
            company: "Tinderbox Robotics",
            location: "Pittsburgh, PA",
            workplace: Workplace::OnSite,
            days_ago: 9,
            applicants: 36,
            description: "Build the pipelines that turn robot sensor logs into \
                          training data. You'll work next to the hardware lab, \
                          so expect the occasional mug to fly past.\n\
                          Experience with streaming systems is a plus.",
            saved: false,
            applied: false,
        },
        Job {
            title: "Engineering Manager, Build Tools",
            company: "Larkspur Labs",
            location: "Toronto, Canada",
            workplace: Workplace::Hybrid,
            days_ago: 14,
            applicants: 64,
            description: "Lead a team of six engineers who own the build system, \
                          the CI fleet and the developer laptop setup. Your goal: \
                          make every engineer's edit-compile-test loop faster \
                          this year than last year.\n\
                          Two days a week in the Toronto office.",
            saved: false,
            applied: false,
        },
    ]
}

fn msg(from_me: bool, hours_ago: u32, body: &'static str) -> Message {
    Message {
        from_me,
        body: body.to_owned(),
        hours_ago,
    }
}

pub(crate) fn conversations() -> Vec<Conversation> {
    vec![
        Conversation {
            name: "Maren Holt",
            headline: "Staff Engineer at Quillfeather Systems",
            messages: vec![
                msg(false, 3, "Did you see the new CI numbers?"),
                msg(true, 3, "Six minutes! I thought the dashboard was broken."),
                msg(
                    false,
                    1,
                    "Ha. Want to pair on the caching write-up this week?",
                ),
            ],
            unread: true,
        },
        Conversation {
            name: "Ravi Tenorio",
            headline: "Engineering Manager at Quillfeather Systems",
            messages: vec![
                msg(
                    false,
                    5,
                    "Quick one: are you free for a coffee chat on Thursday?",
                ),
                msg(false, 5, "No agenda, just catching up."),
            ],
            unread: true,
        },
        Conversation {
            name: "Ines Okonkwo",
            headline: "Technical Recruiter at Harborline Talent",
            messages: vec![
                msg(
                    false,
                    30,
                    "Hi Jordan, thanks for connecting! Let me know if you ever \
                     want to hear about roles at Harborline.",
                ),
                msg(true, 28, "Thanks Ines, I'll keep it in mind."),
            ],
            unread: false,
        },
        Conversation {
            name: "Felix Abernathy",
            headline: "Software Engineering Intern at Quillfeather Systems",
            messages: vec![
                msg(false, 52, "Thanks for reviewing my first PR!"),
                msg(true, 51, "Of course. Great first change."),
                msg(false, 50, "On to the next typo."),
            ],
            unread: false,
        },
    ]
}

fn notification(text: &'static str, hours_ago: u32, unread: bool) -> Notification {
    Notification {
        text,
        hours_ago,
        unread,
        is_new: false,
    }
}

pub(crate) fn notifications() -> Vec<Notification> {
    vec![
        notification("Maren Holt and 41 others reacted to your post", 1, true),
        notification(
            "Your post on terminal layouts passed 1,000 impressions",
            3,
            true,
        ),
        notification("Ines Okonkwo viewed your profile", 5, true),
        notification("You appeared in 12 searches this week", 26, false),
        notification(
            "Congratulate Casey Lindqvist on 5 years at Pinecrest Studio",
            48,
            false,
        ),
    ]
}
