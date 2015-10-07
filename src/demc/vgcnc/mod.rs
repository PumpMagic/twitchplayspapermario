use std::collections::HashMap;

use demc::virtc::*;


//@todo track state
pub struct VGcnC {
    axes: HashMap<String, (u32, i64, i64)>,
    joysticks: HashMap<String, (String, String)>,
    buttons: HashMap<String, u8>,
    device_number: u32
}

impl IsVJoyDevice for VGcnC {
    fn get_device_number(&self) -> u32 {
        self.device_number
    }
}
impl HasAxes for VGcnC {
    fn get_axis_map(&self) -> &HashMap<String, (u32, i64, i64)> {
        &self.axes
    }
}
impl HasJoysticks for VGcnC {
    fn get_joystick_map(&self) -> &HashMap<String, (String, String)> {
        &self.joysticks
    }
}
impl HasButtons for VGcnC {
    fn get_button_map(&self) -> &HashMap<String, u8> {
        &self.buttons
    }
}
impl HasAxesAndButtons for VGcnC {}
impl AcceptsInputs for VGcnC {}

impl VGcnC {
    // Err(1): unable to get N64 hardware
    // Err(2): vjoystick doesn't meet N64 controller requirements
    // Err(3): unable to claim and reset vjoystick
    pub fn new(device_number: u32,
               axes: HashMap<String, (u32, i64, i64)>,
               joysticks: HashMap<String, (String, String)>,
               buttons: HashMap<String, u8>)
                    -> Result<Self, u8>
    {
        let virtc = VGcnC { axes: axes, joysticks: joysticks, buttons: buttons, device_number: device_number };

        match virtc.verify_vjoystick_compatibility() {
            Ok(_) => (),
            Err(_) => return Err(2)
        }

        match virtc.claim_and_reset() {
            Ok(_) => Ok(virtc),
            Err(_) => Err(3)
        }
    }
}

pub fn sample_gcn_controller_hardware(device_number: u32)
        -> Result<(HashMap<String, (u32, i64, i64)>, HashMap<String, (String, String)>,HashMap<String, u8>), u8>
{
    let mut axes = HashMap::new();

    let jx_min = match vjoy_rust::get_vjoystick_axis_min(device_number, 0x30) {
        Ok(min) => min,
        Err(_) => return Err(1)
    };
    let jx_max = match vjoy_rust::get_vjoystick_axis_max(device_number, 0x30) {
        Ok(max) => max,
        Err(_) => return Err(2)
    };
    let jy_min = match vjoy_rust::get_vjoystick_axis_min(device_number, 0x31) {
        Ok(min) => min,
        Err(_) => return Err(3)
    };
    let jy_max = match vjoy_rust::get_vjoystick_axis_max(device_number, 0x31) {
        Ok(max) => max,
        Err(_) => return Err(4)
    };

    let cx_min = match vjoy_rust::get_vjoystick_axis_min(device_number, 0x33) {
        Ok(min) => min,
        Err(_) => return Err(1)
    };
    let cx_max = match vjoy_rust::get_vjoystick_axis_max(device_number, 0x33) {
        Ok(max) => max,
        Err(_) => return Err(2)
    };
    let cy_min = match vjoy_rust::get_vjoystick_axis_min(device_number, 0x34) {
        Ok(min) => min,
        Err(_) => return Err(3)
    };
    let cy_max = match vjoy_rust::get_vjoystick_axis_max(device_number, 0x34) {
        Ok(max) => max,
        Err(_) => return Err(4)
    };

    axes.insert(String::from("jx"), (0x30, jx_min, jx_max));
    axes.insert(String::from("jy"), (0x31, jy_min, jy_max));
    axes.insert(String::from("cx"), (0x33, cx_min, cx_max));
    axes.insert(String::from("cy"), (0x34, cy_min, cy_max));

    let mut joysticks = HashMap::new();
    joysticks.insert(String::from("control_stick"), (String::from("jx"), String::from("jy")));
    joysticks.insert(String::from("c_stick"), (String::from("cx"), String::from("cy")));

    let mut buttons = HashMap::new();
    buttons.insert(String::from("a"), 0x01);
    buttons.insert(String::from("b"), 0x02);
    buttons.insert(String::from("x"), 0x03);
    buttons.insert(String::from("y"), 0x04);
    buttons.insert(String::from("z"), 0x05);
    buttons.insert(String::from("l"), 0x06);
    buttons.insert(String::from("r"), 0x07);
    buttons.insert(String::from("start"), 0x08);
    buttons.insert(String::from("dup"), 0x09);
    buttons.insert(String::from("ddown"), 0x0a);
    buttons.insert(String::from("dleft"), 0x0b);
    buttons.insert(String::from("dright"), 0x0c);

    Ok((axes, joysticks, buttons))
}