use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use demc::virtc::*;


//@todo track state
pub struct VN64C {
    device_number: u32,
    axis_constants: HashMap<String, (u32, i64, i64)>,
    axis_states: Arc<Mutex<HashMap<String, f32>>>,
    joysticks: HashMap<String, (String, String)>,
    button_names_to_indices: HashMap<String, u8>,
    button_names_to_states: Arc<Mutex<HashMap<String, bool>>>
}

impl IsVJoyDevice for VN64C {
    fn get_device_number(&self) -> u32 {
        self.device_number
    }
}
impl HasAxes for VN64C {
    fn get_axis_constants(&self) -> &HashMap<String, (u32, i64, i64)> {
        &self.axis_constants
    }
    
    fn get_axis_states(&self) -> Arc<Mutex<HashMap<String, f32>>> {
        self.axis_states.clone()
    }
}
impl HasJoysticks for VN64C {
    fn get_joystick_map(&self) -> &HashMap<String, (String, String)> {
        &self.joysticks
    }
}
impl HasButtons for VN64C {
    fn get_button_name_to_index_map(&self) -> &HashMap<String, u8> {
        &self.button_names_to_indices
    }
    
    fn get_button_name_to_state_map(&self) -> Arc<Mutex<HashMap<String, bool>>> {
        self.button_names_to_states.clone()
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
               axis_constants: HashMap<String, (u32, i64, i64)>,
               joysticks: HashMap<String, (String, String)>,
               button_names_to_indices: HashMap<String, u8>)
                    -> Result<Self, u8>
    {
        let mut axis_states = HashMap::new();
        for (axis_name, _) in axis_constants.iter() {
            axis_states.insert(axis_name.clone(), 0.0);
        }
    
        let mut button_names_to_states = HashMap::new();
        for (button_name, _) in button_names_to_indices.iter() {
            button_names_to_states.insert(button_name.clone(), false);
        }
        
        let virtc = VN64C { device_number: device_number,
                            axis_constants: axis_constants,
                            axis_states: Arc::new(Mutex::new(axis_states)),
                            joysticks: joysticks,
                            button_names_to_indices: button_names_to_indices,
                            button_names_to_states: Arc::new(Mutex::new(button_names_to_states)) };

        if let Err(_) = virtc.verify_vjoystick_compatibility() {
            return Err(2);
        }

        match virtc.claim_and_reset() {
            Ok(_) => Ok(virtc),
            Err(_) => Err(3)
        }
    }
}

pub fn sample_n64_controller_hardware(device_number: u32)
        -> Result<(HashMap<String, (u32, i64, i64)>, HashMap<String, (String, String)>,HashMap<String, u8>), u8>
{
    let mut axis_constants = HashMap::new();

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
    
    axis_constants.insert(String::from("x"), (0x30, x_min, x_max));
    axis_constants.insert(String::from("y"), (0x31, y_min, y_max));

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

    Ok((axis_constants, joysticks, buttons))
}