use wasm_bindgen::prelude::*;

slint::include_modules!();

#[wasm_bindgen]
pub struct RavenNotchApp {
    pill: Pill,
}

#[wasm_bindgen]
impl RavenNotchApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<RavenNotchApp, JsValue> {
        let pill = Pill::new().map_err(|e| JsValue::from_str(&e.to_string()))?;
        pill.set_motion_width(260.0);
        pill.set_motion_height(38.0);
        pill.set_motion_radius(20.0);
        Ok(RavenNotchApp { pill })
    }

    pub fn show(&self) -> Result<(), JsValue> {
        self.pill.show().map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(())
    }

    pub fn set_telemetry(&self, cpu: f32, ram: f32, gpu: f32) {
        self.pill.set_cpu_pct(cpu);
        self.pill.set_ram_pct(ram);
        self.pill.set_gpu_pct(gpu);
    }

    pub fn set_time_date(&self, time_str: &str, date_str: &str) {
        self.pill.set_time(time_str.into());
        self.pill.set_date(date_str.into());
    }

    pub fn set_battery(&self, pct: f32, charging: bool) {
        self.pill.set_battery_pct(pct);
        self.pill.set_is_charging(charging);
    }

    pub fn set_expanded(&self, expanded: bool) {
        self.pill.set_is_expanded(expanded);
        self.pill.set_panel_ready(expanded);
        self.pill.set_content_opacity(if expanded { 1.0 } else { 0.0 });
        self.pill.set_notch_phase(if expanded { "expanded".into() } else { "closed".into() });
        if expanded {
            self.pill.set_motion_width(720.0);
            self.pill.set_motion_height(244.0);
            self.pill.set_motion_radius(20.0);
        } else {
            self.pill.set_motion_width(260.0);
            self.pill.set_motion_height(38.0);
            self.pill.set_motion_radius(20.0);
        }
    }

    pub fn set_active_tab(&self, tab: &str) {
        self.pill.set_active_tab(tab.into());
    }
}
