use egui::{Color32, FontFamily, FontId, RichText, Vec2, Visuals};
use egui::epaint::CornerRadius;

pub struct Theme {
    pub accent_color: Color32,
    pub background_color: Color32,
    pub panel_color: Color32,
    pub active_color: Color32,
    pub inactive_color: Color32,
    pub text_color: Color32,
    pub dim_text_color: Color32,
    pub header_text_color: Color32,
    pub widget_gap: f32,
    pub heading_font: FontId,
    pub body_font: FontId,
    pub small_font: FontId,
    pub tiny_font: FontId,
    pub corner_radius: CornerRadius, // previously rounding
    pub widget_padding: Vec2,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            accent_color: Color32::from_rgb(94, 129, 172),
            background_color: Color32::from_rgb(46, 52, 64),
            panel_color: Color32::from_rgb(59, 66, 82),
            active_color: Color32::from_rgb(136, 192, 208),
            inactive_color: Color32::from_rgb(76, 86, 106),
            text_color: Color32::from_rgb(236, 239, 244),
            dim_text_color: Color32::from_rgb(216, 222, 233),
            header_text_color: Color32::from_rgb(129, 161, 193),
            widget_gap: 8.0,
            heading_font: FontId::new(26.0, FontFamily::Proportional),
            body_font: FontId::new(16.0, FontFamily::Proportional),
            small_font: FontId::new(14.0, FontFamily::Proportional),
            tiny_font: FontId::new(12.0, FontFamily::Proportional),
            corner_radius: CornerRadius::same(8), // now takes a u8
            widget_padding: Vec2::new(8.0, 6.0),
        }
    }

    pub fn light() -> Self {
        Self {
            accent_color: Color32::from_rgb(94, 129, 172),
            background_color: Color32::from_rgb(236, 239, 244),
            panel_color: Color32::from_rgb(229, 233, 240),
            active_color: Color32::from_rgb(129, 161, 193),
            inactive_color: Color32::from_rgb(216, 222, 233),
            text_color: Color32::from_rgb(59, 66, 82),
            dim_text_color: Color32::from_rgb(76, 86, 106),
            header_text_color: Color32::from_rgb(46, 52, 64),
            widget_gap: 8.0,
            heading_font: FontId::new(26.0, FontFamily::Proportional),
            body_font: FontId::new(16.0, FontFamily::Proportional),
            small_font: FontId::new(14.0, FontFamily::Proportional),
            tiny_font: FontId::new(12.0, FontFamily::Proportional),
            corner_radius: CornerRadius::same(8),
            widget_padding: Vec2::new(8.0, 6.0),
        }
    }

    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        style.spacing.item_spacing = Vec2::new(self.widget_gap, self.widget_gap);
        style.spacing.window_margin = self.widget_padding.into(); // Convert Vec2 to Margin
        style.spacing.button_padding = self.widget_padding;
        
        let mut visuals = Visuals::dark();
        visuals.widgets.noninteractive.bg_fill = self.panel_color;
        visuals.widgets.inactive.bg_fill = self.inactive_color;
        visuals.widgets.active.bg_fill = self.active_color;
        visuals.widgets.hovered.bg_fill = self.accent_color;
        
        visuals.widgets.noninteractive.corner_radius = self.corner_radius; // Correct usage
        visuals.widgets.inactive.corner_radius = self.corner_radius;
        visuals.widgets.active.corner_radius = self.corner_radius;
        visuals.widgets.hovered.corner_radius = self.corner_radius;
        
        visuals.window_corner_radius = self.corner_radius;
        visuals.window_fill = self.panel_color;
        
        style.visuals = visuals;
        ctx.set_style(style);
    }

    pub fn title_text(&self, text: &str) -> RichText {
        RichText::new(text)
            .font(self.heading_font.clone())
            .color(self.header_text_color)
    }

    pub fn heading_text(&self, text: &str) -> RichText {
        RichText::new(text)
            .font(self.body_font.clone())
            .color(self.header_text_color)
            .strong()
    }

    pub fn body_text(&self, text: &str) -> RichText {
        RichText::new(text)
            .font(self.body_font.clone())
            .color(self.text_color)
    }
    
    pub fn secondary_text(&self, text: &str) -> RichText {
        RichText::new(text)
            .font(self.small_font.clone())
            .color(self.dim_text_color)
    }

    pub fn tiny_text(&self, text: &str) -> RichText {
        RichText::new(text)
            .font(self.tiny_font.clone())
            .color(self.dim_text_color)
    }
}
