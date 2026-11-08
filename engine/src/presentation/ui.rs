use super::Settings;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Agents,
    Controls,
    Presentation,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    #[default]
    All,
    Duelist,
    Controller,
    Sentinel,
    Initiator,
}
impl Role {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Duelist,
            Self::Duelist => Self::Controller,
            Self::Controller => Self::Sentinel,
            Self::Sentinel => Self::Initiator,
            Self::Initiator => Self::All,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Duelist => "Duelist",
            Self::Controller => "Controller",
            Self::Sentinel => "Sentinel",
            Self::Initiator => "Initiator",
        }
    }
}
pub fn role(index: usize) -> Role {
    match index {
        0 | 2 | 4 | 5 | 14 | 15 | 16 | 17 => Role::Duelist,
        3 | 6 | 7 | 8 | 9 | 10 => Role::Controller,
        1 | 12 => Role::Sentinel,
        _ => Role::Initiator,
    }
}
#[derive(Clone, Debug)]
pub struct Menu {
    pub tab: Tab,
    pub role: Role,
    pub query: String,
    pub page: usize,
    pub search: bool,
    pub close: bool,
    pub select: Option<usize>,
    pub dirty: bool,
    pub binding: Option<(usize, String)>,
}
impl Default for Menu {
    fn default() -> Self {
        Self {
            tab: Tab::Agents,
            role: Role::All,
            query: String::new(),
            page: 0,
            search: false,
            close: false,
            select: None,
            dirty: false,
            binding: None,
        }
    }
}
impl Menu {
    pub fn results(&self, names: &[&str]) -> Vec<usize> {
        let q = self.query.to_lowercase();
        names
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                (self.role == Role::All || role(*i) == self.role) && n.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn typed(&mut self, c: char) {
        if self.tab == Tab::Agents
            && self.search
            && !c.is_control()
            && self.query.chars().count() < 32
        {
            self.query.push(c);
            self.page = 0;
        }
    }
    pub fn adjust(&mut self, row: usize, s: &mut Settings) {
        match row {
            0 => s.quality = s.quality.next(),
            1 => {
                s.scale += 0.1;
                if s.scale > 1.51 {
                    s.scale = 0.7;
                }
            }
            2 => {
                s.x += 0.1;
                if s.x > 1.01 {
                    s.x = 0.;
                }
            }
            3 => {
                s.y += 0.1;
                if s.y > 1.01 {
                    s.y = 0.;
                }
            }
            4 => s.sound = !s.sound,
            _ => return,
        }
        s.sanitize();
        self.dirty = true;
    }
}
/// Original 8x8 geometric symbols. Semantic glyphs, not Riot's extracted icons.
pub fn icon(name: &str, ultimate: bool) -> [u8; 8] {
    let n = name.to_lowercase();
    if ultimate {
        return [0x18, 0x3c, 0x7e, 0xdb, 0xdb, 0x7e, 0x3c, 0x18];
    }
    if ["smoke", "cloud", "cover", "nebula", "ruse", "pit"]
        .iter()
        .any(|x| n.contains(x))
    {
        [0, 0x18, 0x3c, 0x7e, 0xff, 0xff, 0x7e, 0]
    } else if ["heal", "regrowth", "devour", "pick-me"]
        .iter()
        .any(|x| n.contains(x))
    {
        [0x18, 0x18, 0x7e, 0x7e, 0x7e, 0x18, 0x18, 0]
    } else if [
        "wall",
        "barrier",
        "blaze",
        "tide",
        "lane",
        "divide",
        "contingency",
    ]
    .iter()
    .any(|x| n.contains(x))
    {
        [0xff, 0x89, 0xff, 0x91, 0xff, 0x89, 0xff, 0]
    } else if ["flash", "curve", "blind", "leer", "guiding", "dizzy"]
        .iter()
        .any(|x| n.contains(x))
    {
        [0x18, 0x99, 0x5a, 0x3c, 0x3c, 0x5a, 0x99, 0x18]
    } else if [
        "dash", "wind", "draft", "gear", "step", "crash", "refract", "light",
    ]
    .iter()
    .any(|x| n.contains(x))
    {
        [0x44, 0x66, 0x33, 0x99, 0x33, 0x66, 0x44, 0]
    } else if [
        "recon", "haunt", "cam", "drone", "trail", "seek", "prowler", "wingman",
    ]
    .iter()
    .any(|x| n.contains(x))
    {
        [0, 0x3c, 0x66, 0xdb, 0xdb, 0x66, 0x3c, 0]
    } else if ["fire", "hand", "incend", "mosh", "shell", "frag", "shock"]
        .iter()
        .any(|x| n.contains(x))
    {
        [0x08, 0x18, 0x1c, 0x3e, 0x7e, 0x7e, 0x3c, 0x18]
    } else {
        [0x18, 0x3c, 0x7e, 0xdb, 0x18, 0x18, 0x18, 0]
    }
}

/// Fit even small GUI sizes; drawing and mouse hit testing share this transform.
pub struct Viewport {
    pub size: [f32; 2],
    pub scale: f32,
}
impl Viewport {
    pub fn new(size: [f32; 2]) -> Self {
        let scale = (size[0] / 300.).min(size[1] / 240.).clamp(0.05, 1.);
        Self {
            size: size.map(|v| v / scale),
            scale,
        }
    }
    pub fn point(&self, p: [f32; 2]) -> [f32; 2] {
        p.map(|v| v / self.scale)
    }
}
