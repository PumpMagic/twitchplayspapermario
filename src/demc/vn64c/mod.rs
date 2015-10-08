use std::collections::HashMap;

use demc::virtc::*;

//@todo track state
pub struct VN64C {
    axes: HashMap<String, (u32, i64, i64)>,
    joysticks: HashMap<String, (String, String)>,
    buttons: HashMap<String, u8>,
    device_number: u32
}

impl IsVJoyDevice for VN64C {
    fn get_device_number(&self) -> u32 {
        self.device_number
    }
}
impl HasAxes for VN64C {
    fn get_axis_map(&self) -> &HashMap<String, (u32, i64, i64)> {
        &self.axes
    }
}
impl HasJoysticks for VN64C {
    fn get_joystick_map(&self) -> &HashMap<String, (String, String)> {
        &self.joysticks
    }
}
impl HasButtons for VN64C {
    fn get_button_map(&self) -> &HashMap<String, u8> {
        &self.buttons
    }
}
impl HasAxesAndButtons for VN64C {}
impl AcceptsInputs for VN64C {
    fn set_input(&self, input: &Input) -> Result<(), u8> {
        match input.clone() {
            Input::Joystick(name, direction, strength) => self.set_joystick_state(&name, direction, strength),
            Input::Button(name, value) => self.set_button_state(&name, value)
        }
    }
}

impl VN64C {
    // Err(1): unable to get N64 hardware
    // Err(2): vjoystick doesn't meet N64 controller requirements
    // Err(3): unable to claim and reset vjoystick
    pub fn new(device_number: u32,
               axes: HashMap<String, (u32, i64, i64)>,
               joysticks: HashMap<String, (String, String)>,
               buttons: HashMap<String, u8>)
                    -> Result<Self, u8>
    {
        let virtc = VN64C { axes: axes, joysticks: joysticks, buttons: buttons, device_number: device_number };

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

pub fn sample_n64_controller_hardware(device_number: u32) -> Result<(HashMap<String, (u32, i64, i64)>, HashMap<String, (String, String)>, HashMap<String, u8>), u8> {
    let mut axes = HashMap::new();

    let x_min = match vjoy_rust::get_vjoystick_axis_min(device_number, 0x30) {
        Ok(min) => min,
        Err(_) => return Err(1)
    };
    let x_max = match vjoy_rust::get_vjoystick_axis_max(device_number, 0x30) {
        Ok(max) => max,
        Err(_) => return Err(2)
    };
    let y_min = match vjoy_rust::get_vjoystick_axis_min(device_number, 0x31) {
        Ok(min) => min,
        Err(_) => return Err(3)
    };
    let y_max = match vjoy_rust::get_vjoystick_axis_max(device_number, 0x31) {
        Ok(max) => max,
        Err(_) => return Err(4)
    };

    axes.insert(String::from("x"), (0x30, x_min, x_max));
    axes.insert(String::from("y"), (0x31, y_min, y_max));

    let mut joysticks = HashMap::new();
    joysticks.insert(String::from("control_stick"), (String::from("x"), String::from("y")));

    let mut buttons = HashMap::new();
    buttons.insert(String::from("a"), 0x01);
    buttons.insert(String::from("b"), 0x02);
    buttons.insert(String::from("z"), 0x03);
    buttons.insert(String::from("l"), 0x04);
    buttons.insert(String::from("r"), 0x05);
    buttons.insert(String::from("start"), 0x06);
    buttons.insert(String::from("cup"), 0x07);
    buttons.insert(String::from("cdown"), 0x08);
    buttons.insert(String::from("cleft"), 0x09);
    buttons.insert(String::from("cright"), 0x0a);
    buttons.insert(String::from("dup"), 0x0b);
    buttons.insert(String::from("ddown"), 0x0c);
    buttons.insert(String::from("dleft"), 0x0d);
    buttons.insert(String::from("dright"), 0x0e);

    Ok((axes, joysticks, buttons))
}
