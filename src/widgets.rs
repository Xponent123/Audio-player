use egui::{pos2, Pos2, Rect, Vec2};
use egui::epaint::CornerRadius;
use egui_phosphor::regular::*;
use crate::theme::Theme;

// Custom playback control buttons
pub fn play_button(ui: &mut egui::Ui, is_playing: bool, theme: &Theme) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(42.0, 42.0),
        egui::Sense::click(),
    );
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let is_hovered = response.hovered();
        
        // Draw the background circle
        let bg_color = if is_hovered {
            theme.accent_color
        } else {
            theme.inactive_color
        };
        
        painter.circle_filled(
            rect.center(),
            rect.width() / 2.0,
            bg_color,
        );
        
        // Draw the icon (play or pause)
        let icon_color = theme.text_color;
        if is_playing {
            // Draw pause icon
            let center = rect.center();
            let bar_width = 4.0;
            let bar_height = 14.0;
            let spacing = 6.0;
            
            let pause_left = Rect::from_center_size(
                Pos2::new(center.x - spacing/2.0, center.y),
                Vec2::new(bar_width, bar_height),
            );
            
            let pause_right = Rect::from_center_size(
                Pos2::new(center.x + spacing/2.0, center.y),
                Vec2::new(bar_width, bar_height),
            );
            
            painter.rect_filled(pause_left, CornerRadius::ZERO, icon_color);
            painter.rect_filled(pause_right, CornerRadius::ZERO, icon_color);
        } else {
            // Draw play icon (triangle)
            let center = rect.center();
            let tri_height = 14.0;
            let tri_base = 12.0;
            
            // Draw triangle as a shape using painter.add
            let points = vec![
                Pos2::new(center.x - tri_base/4.0, center.y - tri_height/2.0),
                Pos2::new(center.x - tri_base/4.0, center.y + tri_height/2.0),
                Pos2::new(center.x + tri_base/2.0, center.y),
            ];
            
            // Use add for custom shapes
            painter.add(egui::Shape::convex_polygon(
                points,
                icon_color,
                (0.0, icon_color) // Stroke (unused for filled shape)
            ));
        }
    }
    
    response.clicked()
}

pub fn prev_button(ui: &mut egui::Ui, theme: &Theme) -> bool {
    control_button(ui, SKIP_BACK, "Previous Track", theme)
}

pub fn next_button(ui: &mut egui::Ui, theme: &Theme) -> bool {
    control_button(ui, SKIP_FORWARD, "Next Track", theme)
}

pub fn shuffle_button(ui: &mut egui::Ui, is_active: bool, theme: &Theme) -> bool {
    toggle_button(ui, SHUFFLE, "Shuffle", is_active, theme)
}

fn control_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, theme: &Theme) -> bool {
    let size = Vec2::new(36.0, 36.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    
    let response_clone = response.clone();
    response.on_hover_text(tooltip);
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let is_hovered = response_clone.hovered();
        
        let bg_color = if is_hovered {
            theme.inactive_color
        } else {
            theme.panel_color
        };
        
        painter.circle_filled(
            rect.center(),
            rect.width() / 2.0,
            bg_color,
        );
        
        let icon_size = 16.0;
        let icon_color = theme.text_color;
        
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(icon_size),
            icon_color,
        );
    }
    
    response_clone.clicked()
}

fn toggle_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, is_active: bool, theme: &Theme) -> bool {
    let size = Vec2::new(30.0, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    
    let response_clone = response.clone();
    response.on_hover_text(tooltip);
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let is_hovered = response_clone.hovered();
        
        let bg_color = if is_active {
            theme.accent_color
        } else if is_hovered {
            theme.inactive_color
        } else {
            theme.panel_color
        };
        
        painter.circle_filled(
            rect.center(),
            rect.width() / 2.0,
            bg_color,
        );
        
        let icon_size = 14.0;
        let icon_color = theme.text_color;
        
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(icon_size),
            icon_color,
        );
    }
    
    response_clone.clicked()
}

// Custom volume slider
pub fn volume_slider(ui: &mut egui::Ui, volume: &mut f32, theme: &Theme) -> bool {
    let desired_size = Vec2::new(120.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    
    let mut value_changed = false;
    
    if response.dragged() || response.clicked() {
        let new_volume = ((response.interact_pointer_pos().unwrap_or_else(|| rect.left_top()).x - rect.left()) / rect.width())
            .clamp(0.0, 1.0);
        
        if (*volume - new_volume).abs() > 0.001 {
            *volume = new_volume;
            value_changed = true;
        }
    }
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        
        // Draw track background
        painter.rect_filled(
            rect,
            theme.corner_radius,
            theme.inactive_color,
        );
        
        // Draw filled portion
        let filled_width = rect.width() * *volume;
        if filled_width > 0.0 {
            let filled_rect = Rect::from_min_size(
                rect.left_top(),
                Vec2::new(filled_width, rect.height()),
            );
            painter.rect_filled(
                filled_rect,
                theme.corner_radius,
                theme.accent_color,
            );
        }
        
        // Draw handle
        let handle_radius = 10.0;
        let handle_x = rect.left() + rect.width() * *volume;
        let handle_y = rect.center().y;
        
        painter.circle_filled(
            Pos2::new(handle_x, handle_y),
            handle_radius,
            theme.text_color,
        );
        
        // Draw volume icon
        let icon = if *volume < 0.01 {
            SPEAKER_NONE
        } else if *volume < 0.3 {
            SPEAKER_LOW
        } else if *volume < 0.7 {
            SPEAKER_HIGH
        } else {
            SPEAKER_HIGH
        };
        
        ui.painter().text(
            pos2(rect.left() - 24.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            icon,
            egui::FontId::proportional(16.0),
            theme.text_color,
        );
    }
    
    value_changed
}

// Custom progress bar for playback
pub fn progress_bar(ui: &mut egui::Ui, current: f32, total: f32, theme: &Theme) -> Option<f32> {
    let desired_size = Vec2::new(ui.available_width(), 24.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    
    let mut seek_pos = None;
    
    if response.dragged() || response.clicked() {
        let ratio = ((response.interact_pointer_pos().unwrap_or_else(|| rect.left_top()).x - rect.left()) / rect.width())
            .clamp(0.0, 1.0);
        
        seek_pos = Some(ratio * total);
    }
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        
        // Draw track background
        painter.rect_filled(
            rect,
            theme.corner_radius,
            theme.inactive_color,
        );
        
        // Draw filled portion
        let progress_ratio = if total > 0.0 { current / total } else { 0.0 };
        let filled_width = rect.width() * progress_ratio;
        
        if filled_width > 0.0 {
            let filled_rect = Rect::from_min_size(
                rect.left_top(),
                Vec2::new(filled_width, rect.height()),
            );
            painter.rect_filled(
                filled_rect,
                theme.corner_radius,
                theme.accent_color,
            );
        }
        
        // Draw handle
        let handle_radius = 8.0;
        let handle_x = rect.left() + rect.width() * progress_ratio;
        let handle_y = rect.center().y;
        
        painter.circle_filled(
            Pos2::new(handle_x, handle_y),
            handle_radius,
            theme.text_color,
        );
        
        // Draw time indicators
        let current_time = format_time(current);
        let total_time = format_time(total);
        
        ui.painter().text(
            pos2(rect.left(), rect.bottom() + 8.0),
            egui::Align2::LEFT_TOP,
            current_time,
            theme.small_font.clone(),
            theme.dim_text_color,
        );
        
        ui.painter().text(
            pos2(rect.right(), rect.bottom() + 8.0),
            egui::Align2::RIGHT_TOP,
            total_time,
            theme.small_font.clone(),
            theme.dim_text_color,
        );
    }
    
    seek_pos
}

fn format_time(seconds: f32) -> String {
    let minutes = (seconds / 60.0) as i32;
    let secs = (seconds % 60.0) as i32;
    format!("{:02}:{:02}", minutes, secs)
}

// Album artwork display 
pub fn album_art(ui: &mut egui::Ui, image_data: Option<&[u8]>, theme: &Theme) {
    let size = Vec2::new(200.0, 200.0);
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    
    if ui.is_rect_visible(rect) {
        if let Some(data) = image_data {
            // Try to load image data
            if let Ok(img) = image::load_from_memory(data) {
                let img_size = [img.width() as usize, img.height() as usize];
                let pixels = img.to_rgba8().into_vec();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(img_size, &pixels);
                let texture = ui.ctx().load_texture(
                    "album_art", 
                    color_image,
                    egui::TextureOptions::default()
                );
                
                // Pass a reference to the texture directly.
                ui.image(&texture);
            }
        } else {
            // Draw placeholder
            let painter = ui.painter();
            
            painter.rect_filled(
                rect,
                theme.corner_radius,
                theme.inactive_color,
            );
            
            // Music note icon
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                MUSIC_NOTES,
                egui::FontId::proportional(48.0),
                theme.dim_text_color,
            );
        }
    }
}

// Track entry in a playlist
pub fn track_entry(
    ui: &mut egui::Ui,
    title: &str,
    artist: Option<&str>,
    duration: Option<f32>,
    is_current: bool,
    theme: &Theme,
) -> egui::Response {
    let height = 50.0;
    let width = ui.available_width();
    
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, height),
        egui::Sense::click(),
    );
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        
        // Background
        let bg_color = if is_current {
            theme.accent_color.linear_multiply(0.3)
        } else if response.hovered() {
            theme.inactive_color
        } else {
            theme.panel_color
        };
        
        painter.rect_filled(
            rect,
            theme.corner_radius,
            bg_color,
        );
        
        // Play icon for current track
        if is_current {
            let play_icon_rect = Rect::from_min_size(
                rect.left_top() + Vec2::new(10.0, 0.0),
                Vec2::new(height, height),
            );
            
            ui.painter().text(
                play_icon_rect.center(),
                egui::Align2::CENTER_CENTER,
                PLAY,
                egui::FontId::proportional(16.0),
                theme.header_text_color,
            );
        }
        
        // Title
        let title_string = if is_current {
            format!("> {}", title)
        } else {
            format!("  {}", title)
        };
        
        let title_pos = Pos2::new(
            rect.left() + (if is_current { 40.0 } else { 16.0 }),
            rect.top() + 15.0,
        );
        
        painter.text(
            title_pos,
            egui::Align2::LEFT_TOP,
            &title_string,
            theme.body_font.clone(),
            if is_current { theme.header_text_color } else { theme.text_color },
        );
        
        // Artist (if available)
        if let Some(artist_name) = artist {
            let artist_pos = Pos2::new(
                rect.left() + (if is_current { 40.0 } else { 16.0 }),
                rect.top() + 35.0,
            );
            
            painter.text(
                artist_pos,
                egui::Align2::LEFT_TOP,
                artist_name,
                theme.small_font.clone(),
                theme.dim_text_color,
            );
        }
        
        // Duration (if available)
        if let Some(dur) = duration {
            let time_str = format_time(dur);
            let time_pos = Pos2::new(
                rect.right() - 16.0,
                rect.center().y,
            );
            
            painter.text(
                time_pos,
                egui::Align2::RIGHT_CENTER,
                time_str,
                theme.small_font.clone(),
                theme.dim_text_color,
            );
        }
    }
    
    response
}
