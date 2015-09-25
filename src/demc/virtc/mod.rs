#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
mod vjoy_rust;

extern crate std;

use std::collections::HashMap;


trait IsVJoyDevice {
    fn get_vjoy_device_number(&self) -> u32;
}

trait HasAxes: IsVJoyDevice {
    // Values are HID constant - min - max triplets
    fn get_axis_map(&self) -> &HashMap<String, (u32, i64, i64)>;

    fn get_axis_hid(&self, name: &String) -> Option<u32> {
        match self.get_axis_map().get(name) {
            Some(&(hid, _, _)) => Some(hid),
            None => None
        }
    }

    fn get_axis_min(&self, name: &String) -> Option<i64> {
        match self.get_axis_map().get(name) {
            Some(&(_, min, _)) => Some(min),
            None => None
        }
    }

    fn get_axis_max(&self, name: &String) -> Option<i64> {
        match self.get_axis_map().get(name) {
            Some(&(_, _, max)) => Some(max),
            None => None
        }
    }

    fn get_axis_state(&self, name: &String) -> Option<i64>;

    //@todo change this to raw value, not percent
    fn set_axis_state(&self, name: &String, strength: f32) -> Result<(), &'static str> {
        let hid = self.get_axis_hid(name).unwrap();

        let mid: i64 = ((self.get_axis_max(name).unwrap() - self.get_axis_max(&name).unwrap())/2) as i64;
        let val = mid + (strength * (mid as f32)) as i64;

        match vjoy_rust::set_vjoystick_axis(self.get_vjoy_device_number(), hid, val) {
            Ok(_) => Ok(()),
            Err(_) => Err("Unable to set axis")
        }
    }
}

trait HasButtons: IsVJoyDevice {
    fn get_button_map(&self) -> &HashMap<String, u8>;

    fn get_button_index(&self, name: &String) -> Option<u8> {
        match self.get_button_map().get(name) {
            Some(index) => Some(*index),
            None => None
        }
    }

    fn get_button_state(&self, name: &String) -> Option<bool>;

    fn set_button_state(&self, name: &String, value: bool) -> Result<(), &'static str> {
        let index = self.get_button_index(name).unwrap();

        let valc = value as i32;

        match vjoy_rust::set_vjoystick_button(self.get_vjoy_device_number(), index, valc) {
            Ok(_) => Ok(()),
            Err(_) => Err("Unable to set virtual joystick button")
        }
    }
}

pub trait AcceptsInputs: HasAxes + HasButtons {
    fn set_input(&self, input: &Input) -> Result<(), &'static str> {
        match input.clone() {
            Input::Axis(name, strength) => self.set_axis_state(&name, strength),
            Input::Button(name, value) => self.set_button_state(&name, value)
        }
    }
}


//@todo track state
pub struct VN64C {
    axes: HashMap<String, (u32, i64, i64)>,
    buttons: HashMap<String, u8>,

    vjoy_device_number: u32
}

impl IsVJoyDevice for VN64C {
    fn get_vjoy_device_number(&self) -> u32 {
        return self.vjoy_device_number;
    }
}

impl HasAxes for VN64C {
    fn get_axis_map(&self) -> &HashMap<String, (u32, i64, i64)> {
        return &self.axes;
    }

    fn get_axis_state(&self, name: &String) -> Option<i64> {
        return Some(0); //@todo
    }
}

impl HasButtons for VN64C {
    fn get_button_map(&self) -> &HashMap<String, u8> {
        return &self.buttons;
    }

    fn get_button_state(&self, name: &String) -> Option<bool> {
        return Some(false);
    }
}

impl AcceptsInputs for VN64C {
}

impl VN64C {
    pub fn new(vjoy_device_number: u32) -> Result<Self, String> {
        let vjoy_device_number_native = vjoy_device_number;

        if vjoy_rust::is_vjoy_enabled() == false {
            return Err(format!("vJoy isn't enabled. Have you installed vJoy?"));
        }

        match verify_vjoystick_hardware(vjoy_device_number_native) {
            Ok(()) => (),
            Err(err) => return Err(format!("Virtual joystick {} can't act as an N64 controller: {}.\
                                            You may need to reconfigure your vJoy device",
                                           vjoy_device_number, err))
        }

        match vjoy_rust::claim_vjoystick(vjoy_device_number_native) {
            Err(msg) => return Err(format!("{}", msg)),
            _ => ()
        }

        match vjoy_rust::reset_vjoystick(vjoy_device_number_native) {
            Err(_) => return Err(format!("Unable to reset joystick")),
            _ => ()
        }

        let (axes, buttons) = match get_n64_controller_hardware(vjoy_device_number) {
            Ok((axes, buttons)) => (axes, buttons),
            Err(_) => return Err(format!("Getting N64 controller hardware failed"))
        };

        Ok( VN64C { axes: axes, buttons: buttons, vjoy_device_number: vjoy_device_number } )
    }
}

fn get_n64_controller_hardware(vjoy_device_number: u32) -> Result<(HashMap<String, (u32, i64, i64)>, HashMap<String, u8>), u8> {
    let mut axes = HashMap::new();

    let x_min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, 0x30) {
        Ok(min) => min,
        Err(msg) => return Err(1)
    };
    let x_max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, 0x30) {
        Ok(max) => max,
        Err(_) => return Err(2)
    };
    let y_min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, 0x31) {
        Ok(min) => min,
        Err(msg) => return Err(3)
    };
    let y_max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, 0x31) {
        Ok(max) => max,
        Err(_) => return Err(4)
    };

    axes.insert(String::from("x"), (0x30, x_min, x_max));
    axes.insert(String::from("y"), (0x31, y_min, y_max));

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

    Ok((axes, buttons))
}

/*
fn get_gcn_controller_hardware() -> ControllerInputs {
    let mut axes = HashMap::new();
    axes.insert(String::from("jx"), 0x30); // USB HID: X
    axes.insert(String::from("jy"), 0x31); // USB HID: Y
    axes.insert(String::from("cx"), 0x33); // USB HID: Rx
    axes.insert(String::from("cy"), 0x34); // USB HID: Ry

    let mut buttons = HashMap::new();
    buttons.insert(String::from("a"), 0x01);
    buttons.insert(String::from("b"), 0x02);
    buttons.insert(String::from("x"), 0x03);
    buttons.insert(String::from("y"), 0x04);
    buttons.insert(String::from("z"), 0x05);
    buttons.insert(String::from("l"), 0x06);
    buttons.insert(String::from("r"), 0x07);
    buttons.insert(String::from("dup"), 0x08);
    buttons.insert(String::from("ddown"), 0x09);
    buttons.insert(String::from("dleft"), 0x0a);
    buttons.insert(String::from("dright"), 0x0b);

    ControllerInputs { axes: axes, buttons: buttons }
}
*/

#[derive(Clone)]
pub enum Input {
    Axis(String, f32),
    Button(String, bool)
}


//@todo implement
fn verify_vjoystick_hardware(index: u32) -> Result<(), String> {
    if vjoy_rust::get_vjoystick_axis_exists(index, 0x30)== false {
        return Err(format!("No X axis"));
    }

    if vjoy_rust::get_vjoystick_axis_exists(index, 0x31)== false {
        return Err(format!("No Y axis"));
    }

    if vjoy_rust::get_vjoystick_button_count(index) < 14 {
        return Err(format!("Less than {} buttons", 14));
    }

    Ok(())
}
