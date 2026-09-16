// specs/003_hue_control.md
//! The seven Hue Control actions and the bridge behaviour they share.

mod bridge;
mod icon;
mod settings;

use std::{collections::HashMap, sync::Arc};

use bridge::{Bridge, BridgeError};
use openaction::global_events::{
    DidReceiveGlobalSettingsEvent, GlobalEventHandler, set_global_event_handler,
};
use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_global_settings, register_action, run,
};
use serde_json::{Value, json};
use settings::{
    GlobalSettings, Settings, Target, normalize_hex, take_index, warmth_to_ct, warmth_to_kelvin,
};
use tokio::sync::{Mutex, RwLock};

/// Why a button press could not reach the bridge.
enum Fail {
    Unpaired,
    NoTarget,
    Bridge(BridgeError),
}

/// The action variants: one UUID each, one shared machinery.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Power,
    Color,
    Cycle,
    Brightness,
    BrightnessRel,
    Temperature,
    Scene,
}

/// Per-instance state that outlives a single press.
#[derive(Default)]
struct InstanceState {
    cursor: usize,
}

/// Behaviour shared by every Hue action.
struct Core {
    instances: Mutex<HashMap<String, InstanceState>>,
    global: Arc<RwLock<GlobalSettings>>,
}

impl Default for Core {
    fn default() -> Self {
        Self {
            instances: Mutex::default(),
            global: Arc::new(RwLock::new(GlobalSettings::default())),
        }
    }
}

impl Core {
    /// Resolve the configured bridge and target for an instance.
    async fn resolve(&self, settings: &Settings) -> Result<(Bridge, Target), Fail> {
        let target = Target::decode(&settings.target).ok_or(Fail::NoTarget)?;
        let global = self.global.read().await;
        let config = global.pick(&settings.bridge).ok_or(Fail::Unpaired)?;
        let bridge = Bridge::new(&config.ip, &config.username).map_err(Fail::Bridge)?;
        Ok((bridge, target))
    }

    /// Render an error face and alert the user.
    async fn fail(&self, instance: &Instance, fail: Fail) -> OpenActionResult<()> {
        let label = match &fail {
            Fail::Unpaired => "Set up",
            Fail::NoTarget => "No target",
            Fail::Bridge(BridgeError::Api { kind: 1, .. }) => "Auth",
            Fail::Bridge(error) => {
                eprintln!("hue-control: {error}");
                "Offline"
            }
        };
        instance.set_image(Some(icon::error(label)), None).await?;
        instance.show_alert().await
    }

    /// The face an action shows before anything is pressed.
    fn static_image(kind: Kind, settings: &Settings) -> String {
        if Target::decode(&settings.target).is_none() {
            return icon::status("Set up");
        }
        match kind {
            Kind::Power => icon::status("Hue"),
            Kind::Color => icon::color(&normalize_hex(&settings.color)),
            Kind::Cycle => {
                let palette = settings.effective_colors();
                icon::cycle(&palette[0], 0, palette.len())
            }
            Kind::Brightness => icon::brightness(settings.brightness.min(100)),
            Kind::BrightnessRel => {
                icon::brightness_relative(settings.brightness_rel.clamp(-50, 50))
            }
            Kind::Temperature => {
                icon::temperature(warmth_to_kelvin(settings.temperature.clamp(1, 100)))
            }
            Kind::Scene => {
                if settings.scene.trim().is_empty() {
                    icon::status("Pick scene")
                } else {
                    icon::scene()
                }
            }
        }
    }

    async fn appear(
        &self,
        kind: Kind,
        instance: &Instance,
        settings: &Settings,
    ) -> OpenActionResult<()> {
        if kind == Kind::Power {
            let image = match self.resolve(settings).await {
                Ok((bridge, target)) => match bridge.read_state(&target).await {
                    Ok(state) => {
                        icon::power(state.get("on").and_then(Value::as_bool).unwrap_or(false))
                    }
                    Err(_) => icon::error("Offline"),
                },
                Err(Fail::Unpaired | Fail::NoTarget) => icon::status("Set up"),
                Err(Fail::Bridge(_)) => icon::error("Offline"),
            };
            return instance.set_image(Some(image), None).await;
        }
        instance
            .set_image(Some(Self::static_image(kind, settings)), None)
            .await
    }

    /// Toggle the target's power, rendering the resulting state.
    async fn toggle_power(&self, instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
        let (bridge, target) = match self.resolve(settings).await {
            Ok(resolved) => resolved,
            Err(fail) => return self.fail(instance, fail).await,
        };
        let state = match bridge.read_state(&target).await {
            Ok(state) => state,
            Err(error) => return self.fail(instance, Fail::Bridge(error)).await,
        };
        let on = state.get("on").and_then(Value::as_bool).unwrap_or(false);
        match bridge.set_state(&target, json!({ "on": !on })).await {
            Ok(()) => instance.set_image(Some(icon::power(!on)), None).await,
            Err(error) => self.fail(instance, Fail::Bridge(error)).await,
        }
    }

    /// Apply an action's configured value to its target.
    async fn press(
        &self,
        kind: Kind,
        instance: &Instance,
        settings: &Settings,
    ) -> OpenActionResult<()> {
        if kind == Kind::Power {
            return self.toggle_power(instance, settings).await;
        }

        let (bridge, target) = match self.resolve(settings).await {
            Ok(resolved) => resolved,
            Err(fail) => return self.fail(instance, fail).await,
        };
        if kind == Kind::Scene && !matches!(target, Target::Group(_)) {
            return self.fail(instance, Fail::NoTarget).await;
        }

        let outcome: Result<String, BridgeError> = match kind {
            Kind::Power => unreachable!("power is handled above"),
            Kind::Color => {
                let hex = normalize_hex(&settings.color);
                match settings::hex_to_xy(&hex) {
                    Some(xy) => bridge
                        .set_state(&target, json!({ "on": true, "xy": xy }))
                        .await
                        .map(|()| icon::color(&hex)),
                    None => Err(BridgeError::Shape("invalid color".to_owned())),
                }
            }
            Kind::Cycle => {
                let palette = settings.effective_colors();
                let index = {
                    let mut instances = self.instances.lock().await;
                    let state = instances.entry(instance.instance_id.clone()).or_default();
                    take_index(&mut state.cursor, palette.len())
                };
                let hex = palette[index].clone();
                match settings::hex_to_xy(&hex) {
                    Some(xy) => bridge
                        .set_state(&target, json!({ "on": true, "xy": xy }))
                        .await
                        .map(|()| icon::cycle(&hex, index, palette.len())),
                    None => Err(BridgeError::Shape("invalid color".to_owned())),
                }
            }
            Kind::Brightness => {
                let percent = settings.brightness.min(100);
                let bri = settings::percent_to_bri(percent);
                bridge
                    .set_state(&target, json!({ "on": true, "bri": bri }))
                    .await
                    .map(|()| icon::brightness(percent))
            }
            Kind::BrightnessRel => {
                let steps = settings.brightness_rel.clamp(-50, 50);
                let increment = (f32::from(steps) * 2.54).round() as i32;
                let increment = increment.clamp(-254, 254);
                let applied = if increment == 0 {
                    Ok(())
                } else {
                    bridge
                        .set_state(&target, json!({ "on": true, "bri_inc": increment }))
                        .await
                };
                applied.map(|()| icon::brightness_relative(steps))
            }
            Kind::Temperature => {
                let warmth = settings.temperature.clamp(1, 100);
                bridge
                    .set_state(&target, json!({ "on": true, "ct": warmth_to_ct(warmth) }))
                    .await
                    .map(|()| icon::temperature(warmth_to_kelvin(warmth)))
            }
            Kind::Scene => {
                let scene = settings.scene.trim().to_owned();
                if scene.is_empty() {
                    Err(BridgeError::Shape("no scene configured".to_owned()))
                } else {
                    bridge
                        .set_state(&target, json!({ "scene": scene }))
                        .await
                        .map(|()| icon::scene())
                }
            }
        };

        match outcome {
            Ok(image) => instance.set_image(Some(image), None).await,
            Err(error) => self.fail(instance, Fail::Bridge(error)).await,
        }
    }

    /// Adjust a value from a dial detent, persisting the new setting.
    async fn rotate(
        &self,
        kind: Kind,
        instance: &Instance,
        settings: &Settings,
        ticks: i16,
    ) -> OpenActionResult<()> {
        let delta = ticks.saturating_mul(settings.tick_scale());
        let mut next = settings.clone();
        match kind {
            Kind::Brightness => {
                next.brightness = (i16::from(settings.brightness) + delta).clamp(1, 100) as u8;
            }
            Kind::Temperature => {
                let current = i32::from(settings.temperature);
                next.temperature = (current + i32::from(delta)).clamp(1, 100) as u16;
            }
            Kind::BrightnessRel => {
                next.brightness_rel = (settings.brightness_rel + delta).clamp(-50, 50);
            }
            _ => return Ok(()),
        }
        instance.set_settings(&next).await?;
        self.press(kind, instance, &next).await
    }

    /// Pressing a dial toggles power, matching a plain button press.
    async fn knob_press(
        &self,
        kind: Kind,
        instance: &Instance,
        settings: &Settings,
    ) -> OpenActionResult<()> {
        match kind {
            Kind::Brightness | Kind::Temperature => self.toggle_power(instance, settings).await,
            _ => Ok(()),
        }
    }

    /// Answer a property-inspector request. All bridge I/O happens here, in Rust.
    async fn pi_request(
        &self,
        kind: Kind,
        instance: &Instance,
        settings: &Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        match payload
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "discover" => {
                let bridges = bridge::discover().await.unwrap_or_default();
                instance
                    .send_to_property_inspector(json!({ "event": "bridges", "bridges": bridges }))
                    .await
            }
            "pair" => {
                let ip = payload
                    .get("ip")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match bridge::pair(ip).await {
                    Ok(username) => {
                        let id = bridge::config_id(ip).await.unwrap_or_default();
                        instance
                            .send_to_property_inspector(
                                json!({ "event": "paired", "ip": ip, "id": id, "username": username }),
                            )
                            .await
                    }
                    // Hue answers error 101 while the bridge is reachable but no link-button
                    // press is pending. Report that as "waiting", not as a failure: the
                    // inspector keeps polling until the button is pressed.
                    Err(error) if error.link_button_pending() => {
                        eprintln!("hue-control: {error}");
                        instance
                            .send_to_property_inspector(json!({ "event": "pairPending", "ip": ip }))
                            .await
                    }
                    Err(error) => {
                        instance
                            .send_to_property_inspector(
                                json!({ "event": "pairError", "message": error.to_string() }),
                            )
                            .await
                    }
                }
            }
            "getTargets" => {
                let ip = payload
                    .get("ip")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let username = payload
                    .get("username")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match Bridge::new(ip, username) {
                    Ok(ready) => {
                        match ready.targets().await {
                            Ok(targets) => {
                                instance
                                    .send_to_property_inspector(
                                        json!({ "event": "targets", "targets": targets }),
                                    )
                                    .await
                            }
                            Err(error) => instance
                                .send_to_property_inspector(
                                    json!({ "event": "targetError", "message": error.to_string() }),
                                )
                                .await,
                        }
                    }
                    Err(error) => {
                        instance
                            .send_to_property_inspector(
                                json!({ "event": "targetError", "message": error.to_string() }),
                            )
                            .await
                    }
                }
            }
            "valueChanged" => self.appear(kind, instance, settings).await,
            _ => Ok(()),
        }
    }

    /// Forget an instance's state when it leaves the surface.
    async fn forget(&self, instance: &Instance) {
        self.instances.lock().await.remove(&instance.instance_id);
    }
}

/// Keeps the plugin's copy of the shared bridge credentials current.
struct GlobalHandler {
    global: Arc<RwLock<GlobalSettings>>,
}

#[async_trait]
impl GlobalEventHandler for GlobalHandler {
    async fn plugin_ready(&self) -> OpenActionResult<()> {
        get_global_settings().await
    }

    async fn did_receive_global_settings(
        &self,
        event: DidReceiveGlobalSettingsEvent,
    ) -> OpenActionResult<()> {
        if let Ok(settings) = serde_json::from_value::<GlobalSettings>(event.payload.settings) {
            *self.global.write().await = settings;
        }
        Ok(())
    }
}

/// Declare one action type per UUID, all delegating to [`Core`].
macro_rules! declare_actions {
    ($($ty:ident => ($uuid:literal, $kind:expr)),+ $(,)?) => {
        $(
            struct $ty {
                core: Arc<Core>,
            }

            #[async_trait]
            impl Action for $ty {
                const UUID: &'static str = $uuid;
                type Settings = Settings;

                async fn will_appear(
                    &self,
                    instance: &Instance,
                    settings: &Settings,
                ) -> OpenActionResult<()> {
                    self.core.appear($kind, instance, settings).await
                }

                async fn will_disappear(
                    &self,
                    instance: &Instance,
                    _settings: &Settings,
                ) -> OpenActionResult<()> {
                    self.core.forget(instance).await;
                    Ok(())
                }

                async fn key_up(
                    &self,
                    instance: &Instance,
                    settings: &Settings,
                ) -> OpenActionResult<()> {
                    self.core.press($kind, instance, settings).await
                }

                async fn dial_rotate(
                    &self,
                    instance: &Instance,
                    settings: &Settings,
                    ticks: i16,
                    _pressed: bool,
                ) -> OpenActionResult<()> {
                    self.core.rotate($kind, instance, settings, ticks).await
                }

                async fn dial_down(
                    &self,
                    instance: &Instance,
                    settings: &Settings,
                ) -> OpenActionResult<()> {
                    self.core.knob_press($kind, instance, settings).await
                }

                async fn did_receive_settings(
                    &self,
                    instance: &Instance,
                    settings: &Settings,
                ) -> OpenActionResult<()> {
                    self.core.appear($kind, instance, settings).await
                }

                async fn property_inspector_did_appear(
                    &self,
                    _instance: &Instance,
                    _settings: &Settings,
                ) -> OpenActionResult<()> {
                    get_global_settings().await
                }

                async fn send_to_plugin(
                    &self,
                    instance: &Instance,
                    settings: &Settings,
                    payload: &Value,
                ) -> OpenActionResult<()> {
                    self.core.pi_request($kind, instance, settings, payload).await
                }
            }
        )+

        async fn register_all(core: Arc<Core>) {
            $( register_action($ty { core: Arc::clone(&core) }).await; )+
        }
    };
}

declare_actions! {
    PowerAction => ("io.github.mario.huecontrol.power", Kind::Power),
    ColorAction => ("io.github.mario.huecontrol.color", Kind::Color),
    CycleAction => ("io.github.mario.huecontrol.cycle", Kind::Cycle),
    BrightnessAction => ("io.github.mario.huecontrol.brightness", Kind::Brightness),
    BrightnessRelAction => ("io.github.mario.huecontrol.brightness-rel", Kind::BrightnessRel),
    TemperatureAction => ("io.github.mario.huecontrol.temperature", Kind::Temperature),
    SceneAction => ("io.github.mario.huecontrol.scene", Kind::Scene),
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let core = Arc::new(Core::default());
    let handler: &'static GlobalHandler = Box::leak(Box::new(GlobalHandler {
        global: Arc::clone(&core.global),
    }));
    set_global_event_handler(handler);
    register_all(core).await;
    run(std::env::args().collect()).await
}
