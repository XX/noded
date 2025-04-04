use eframe::{App, CreationContext};
use egui::{Id, Key, LayerId, Order, Sense, UiBuilder};
use egui_snarl::Snarl;
use egui_snarl::ui::{NodeLayout, PinPlacement, SnarlStyle, SnarlWidget};
use serde::{Deserialize, Serialize};

use crate::node::Node;
use crate::node::viewer::NodeViewer;

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
    pub visible_settings: bool,
    pub edit_mode: EditMode,
    pub editing_nodes_opacity: f32,
    pub viewing_nodes_opacity: f32,
    pub show_nodes: bool,
    pub animation_time: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            visible_settings: false,
            edit_mode: EditMode::Editing,
            editing_nodes_opacity: 1.0,
            viewing_nodes_opacity: 0.5,
            show_nodes: true,
            animation_time: 0.2,
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

        let viewer = NodeViewer::new(cx.wgpu_render_state.clone().expect("WGPU must be enabled"), &snarl);
        Self {
            snarl,
            style,
            settings,
            viewer,
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

        if ctx.input(|i| i.key_pressed(Key::N)) {
            self.settings.visible_settings = !self.settings.visible_settings;
        }

        if self.settings.visible_settings {
            egui::SidePanel::left("style").show(ctx, |ui| {
                // use egui_scale::EguiScale;
                // ui.style_mut().scale(2.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui_probe::Probe::new(&mut self.style).show(ui);
                    egui_probe::Probe::new(&mut self.settings).show(ui);
                });
            });
        }

        ctx.style_mut(|style| style.animation_time = self.settings.animation_time);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.input(|i| {
                if i.key_pressed(Key::Tab) {
                    self.settings.edit_mode.switch();
                }
                if i.key_pressed(Key::H) {
                    self.settings.show_nodes = !self.settings.show_nodes;
                }
            });

            let last_panel_rect = ui.min_rect();

            // Render area in the background
            let render_area_ui = ui.new_child(
                UiBuilder::new()
                    .layer_id(LayerId::new(Order::Background, Id::new("render_area")))
                    .max_rect(last_panel_rect)
                    .sense(Sense::empty()),
            );
            self.viewer
                .draw(&last_panel_rect, render_area_ui.painter(), &mut self.snarl);

            if self.settings.show_nodes {
                // Editing area with nodes in the middle
                let mut editing_area_ui = ui.new_child(
                    UiBuilder::new()
                        .layer_id(LayerId::new(Order::Middle, Id::new("editing_area")))
                        .max_rect(last_panel_rect)
                        .sense(Sense::empty()),
                );

                editing_area_ui.set_max_size(last_panel_rect.size());

                let opacity = match self.settings.edit_mode {
                    EditMode::Editing => self.settings.editing_nodes_opacity,
                    EditMode::View => self.settings.viewing_nodes_opacity,
                };
                editing_area_ui.set_opacity(opacity);

                SnarlWidget::new().id(Id::new("noded")).style(self.style).show(
                    &mut self.snarl,
                    &mut self.viewer,
                    &mut editing_area_ui,
                );
            }

            if let EditMode::View = self.settings.edit_mode {
                // Overlay mouse blocker in the foreground
                let render_area_ui = ui.new_child(
                    UiBuilder::new()
                        .layer_id(LayerId::new(Order::Foreground, Id::new("overlay_area")))
                        .max_rect(last_panel_rect)
                        .sense(Sense::empty()),
                );
                let overlay_response =
                    render_area_ui.interact(last_panel_rect, Id::new("overlay_blocker"), Sense::click_and_drag());

                self.viewer.after_show(ui, &overlay_response, &mut self.snarl);
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
            outer_margin: egui::Margin::ZERO,
            corner_radius: egui::CornerRadius::ZERO,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
            shadow: egui::Shadow::NONE,
        }),
        crisp_magnified_text: Some(true),
        ..SnarlStyle::new()
    }
}
