use eframe::{App, CreationContext};
use egui::{Area, Id, Key, Sense};
use egui_snarl::Snarl;
use egui_snarl::ui::{NodeLayout, PinPlacement, SnarlStyle, SnarlWidget};
use serde::{Deserialize, Serialize};

use crate::node::Node;
use crate::node::viewer::NodeViewer;
use crate::types::Color;

#[derive(Debug, Copy, Clone, Deserialize, Serialize, egui_probe::EguiProbe)]
pub enum EditMode {
    Editing,
    View,
}

impl EditMode {
    pub fn switch(&mut self) -> Self {
        match self {
            Self::Editing => *self = Self::View,
            Self::View => *self = Self::Editing,
        }
        *self
    }
}

#[derive(Debug, Deserialize, Serialize, egui_probe::EguiProbe)]
pub struct AppSettings {
    pub edit_mode: EditMode,
    pub show_nodes: bool,
    pub animation_time: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            edit_mode: EditMode::Editing,
            show_nodes: true,
            animation_time: 10.0,
        }
    }
}

pub struct NodedApp {
    snarl: Snarl<Node>,
    style: SnarlStyle,
    settings: AppSettings,
    viewer: NodeViewer,
}

impl NodedApp {
    pub fn new(cx: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&cx.egui_ctx);

        cx.egui_ctx.style_mut(|style| style.animation_time *= 10.0);

        let snarl = cx.storage.map_or_else(Snarl::new, |storage| {
            storage
                .get_string("snarl")
                .and_then(|snarl| serde_json::from_str(&snarl).ok())
                .unwrap_or_default()
        });
        // let snarl = Snarl::new();

        let style = cx.storage.map_or_else(default_style, |storage| {
            storage
                .get_string("style")
                .and_then(|style| serde_json::from_str(&style).ok())
                .unwrap_or_else(default_style)
        });
        // let style = default_style();

        let settings = cx.storage.map_or_else(AppSettings::default, |storage| {
            storage
                .get_string("settings")
                .and_then(|settings| serde_json::from_str(&settings).ok())
                .unwrap_or_default()
        });

        NodedApp {
            snarl,
            style,
            settings,
            viewer: NodeViewer::new(cx),
        }
    }
}

impl App for NodedApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ctx.set_zoom_factor(1.0);
        // ctx.set_transform_layer(egui::LayerId::background(), egui::emath::TSTransform::from_scaling(1.0));

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_switch(ui);

                if ui.button("Clear All").clicked() {
                    self.snarl = Snarl::default();
                }
            });
        });

        egui::SidePanel::left("style").show(ctx, |ui| {
            // use egui_scale::EguiScale;
            // ui.style_mut().scale(2.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui_probe::Probe::new(&mut self.style).show(ui);
                egui_probe::Probe::new(&mut self.settings).show(ui);
            });
        });

        ctx.style_mut(|style| style.animation_time = self.settings.animation_time);

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(Key::Tab)) {
                self.settings.edit_mode.switch();
            }

            match self.settings.edit_mode {
                EditMode::Editing => ui.set_opacity(1.0),
                EditMode::View => ui.set_opacity(0.3),
            }

            SnarlWidget::new()
                .id(Id::new("noded"))
                .style(self.style)
                .show(&mut self.snarl, &mut self.viewer, ui);

            ui.set_opacity(1.0);

            let last_panel_rect = ui.min_rect();

            if let EditMode::View = self.settings.edit_mode {
                let overlay_response = Area::new(Id::new("overlay_area"))
                    .fixed_pos(last_panel_rect.left_top())
                    .order(egui::Order::Foreground)
                    .interactable(false)
                    .show(ui.ctx(), |ui| {
                        let blocker_response =
                            ui.interact(last_panel_rect, Id::new("overlay_blocker"), Sense::click_and_drag());

                        ui.painter().rect_filled(last_panel_rect, 0.0, Color::TRANSPARENT);
                        blocker_response
                    })
                    .inner;

                self.viewer.after_show(ui, &overlay_response);
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let snarl = serde_json::to_string(&self.snarl).unwrap();
        storage.set_string("snarl", snarl);

        let style = serde_json::to_string(&self.style).unwrap();
        storage.set_string("style", style);

        let settings = serde_json::to_string(&self.settings).unwrap();
        storage.set_string("settings", settings);
    }
}

const fn default_style() -> SnarlStyle {
    SnarlStyle {
        node_layout: Some(NodeLayout::FlippedSandwich),
        pin_placement: Some(PinPlacement::Edge),
        pin_size: Some(7.0),
        node_frame: Some(egui::Frame {
            inner_margin: egui::Margin::same(8),
            outer_margin: egui::Margin {
                left: 0,
                right: 0,
                top: 0,
                bottom: 4,
            },
            corner_radius: egui::CornerRadius::same(8),
            fill: egui::Color32::from_gray(30),
            stroke: egui::Stroke::NONE,
            shadow: egui::Shadow::NONE,
        }),
        bg_frame: Some(egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin::same(2),
            corner_radius: egui::CornerRadius::ZERO,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
            shadow: egui::Shadow::NONE,
        }),
        crisp_magnified_text: Some(true),
        ..SnarlStyle::new()
    }
}
