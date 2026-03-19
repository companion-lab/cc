use opentui_rust::color::Rgba;

pub struct Theme {
    pub bg_dark: Rgba,
    pub bg_panel: Rgba,
    pub bg_input: Rgba,
    pub bg_highlight: Rgba,
    pub accent_primary: Rgba,
    pub accent_secondary: Rgba,
    pub accent_warning: Rgba,
    pub text_primary: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    pub border: Rgba,
    pub border_focus: Rgba,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg_dark: Rgba::from_hex("#0a0a0a").unwrap(),
            bg_panel: Rgba::from_hex("#0f0f0f").unwrap(),
            bg_input: Rgba::from_hex("#121212").unwrap(),
            bg_highlight: Rgba::from_hex("#1a1a1a").unwrap(),
            accent_primary: Rgba::from_hex("#fab283").unwrap(),
            accent_secondary: Rgba::from_hex("#2db84d").unwrap(),
            accent_warning: Rgba::from_hex("#e6b450").unwrap(),
            text_primary: Rgba::from_hex("#eeeeee").unwrap(),
            text_secondary: Rgba::from_hex("#a0a0a0").unwrap(),
            text_muted: Rgba::from_hex("#606060").unwrap(),
            border: Rgba::from_hex("#2a2a2a").unwrap(),
            border_focus: Rgba::from_hex("#3a3a3a").unwrap(),
        }
    }
}

pub struct SyntaxColors {
    pub comment: Rgba,
    pub keyword: Rgba,
    pub function: Rgba,
    pub variable: Rgba,
    pub string: Rgba,
    pub number: Rgba,
}

impl SyntaxColors {
    pub fn dark() -> Self {
        Self {
            comment: Rgba::from_hex("#6a737d").unwrap(),
            keyword: Rgba::from_hex("#f97583").unwrap(),
            function: Rgba::from_hex("#b392f0").unwrap(),
            variable: Rgba::from_hex("#79b8ff").unwrap(),
            string: Rgba::from_hex("#9ecbff").unwrap(),
            number: Rgba::from_hex("#79b8ff").unwrap(),
        }
    }
}
