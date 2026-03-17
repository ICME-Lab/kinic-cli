use crate::ui::TabId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Types,
    Functions,
    Modules,
    Crates,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Types, Tab::Functions, Tab::Modules, Tab::Crates]
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Types => 0,
            Tab::Functions => 1,
            Tab::Modules => 2,
            Tab::Crates => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Tab::Types,
            1 => Tab::Functions,
            2 => Tab::Modules,
            _ => Tab::Crates,
        }
    }

    pub fn from_id(id: &TabId) -> Option<Self> {
        match id.0.as_str() {
            "types" => Some(Tab::Types),
            "functions" => Some(Tab::Functions),
            "modules" => Some(Tab::Modules),
            "crates" => Some(Tab::Crates),
            _ => None,
        }
    }

    pub fn id(&self) -> TabId {
        let id = match self {
            Tab::Types => "types",
            Tab::Functions => "functions",
            Tab::Modules => "modules",
            Tab::Crates => "crates",
        };
        TabId::new(id)
    }

    pub fn next(&self) -> Self {
        Self::from_index(self.index() + 1)
    }

    pub fn prev(&self) -> Self {
        Self::from_index(self.index().wrapping_sub(1).min(3))
    }
}
