#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
mod vjoy_rust;

extern crate std;

use std::collections::HashMap;


// IsVJoyDevice says that the implementor is a representation of a vJoy virtual joystick
trait IsVJoyDevice {
    fn get_vjoy_device_number(&self) -> u32;

    // Convenience function for claiming and resetting the vJoy device with the given number
    // Err(1): vJoy isn't enabled
    // Err(2): Unable to claim vjoystick
    // Err(3): Unable to reset vjoystick
    fn claim_and_reset(&self) -> Result<(), u8> {
        if vjoy_rust::is_vjoy_enabled() == false {
            return Err(1);
        }

        match vjoy_rust::claim_vjoystick(self.get_vjoy_device_number()) {
            Err(msg) => return Err(2),
            _ => ()
        }

        match vjoy_rust::reset_vjoystick(self.get_vjoy_device_number()) {
            Err(_) => return Err(3),
            _ => Ok(())
        }
    }
}

// HasAxes says that the implementor contains at least one vJoy virtual axis
trait HasAxes: IsVJoyDevice {
    // Map of axis names to (vJoy axis HID constant, minimum value, maximum value) triplets
    fn get_axis_map(&self) -> &HashMap<String, (u32, i64, i64)>;

    // Convenience function for getting the vJoy axis HID constant of the axis with given name
    fn get_axis_hid(&self, name: &String) -> Option<u32> {
        match self.get_axis_map().get(name) {
            Some(&(hid, _, _)) => Some(hid),
            None => None
        }
    }

    // Convenience function for getting the vJoy axis minimum of the axis with given name
    fn get_axis_min(&self, name: &String) -> Option<i64> {
        match self.get_axis_map().get(name) {
            Some(&(_, min, _)) => Some(min),
            None => None
        }
    }

    // Convenience function for getting the vJoy axis maximum of the axis with given name
    fn get_axis_max(&self, name: &String) -> Option<i64> {
        match self.get_axis_map().get(name) {
            Some(&(_, _, max)) => Some(max),
            None => None
        }
    }

    // Get the current value of the axis with given name
    fn get_axis_state(&self, name: &String) -> Option<i64>;

    // Set the value of the axis with given name
    // This function takes in a strength argument rather than a raw value so that callers don't need to be aware of
    // the relevant axis' value range. strength must be a number in the range [-1.0, 1.0]
    // Err(1): strength argument invalid
    // Err(2): axis HID not available
    // Err(3): axis min or max not available
    // Err(4): setting axis value failed
    fn set_axis_state(&self, name: &String, strength: f32) -> Result<(), u8> {
        if strength < -1.0 || strength > 1.0 {
            return Err(1);
        }

        let hid = match self.get_axis_hid(name) {
            Some(hid) => hid,
            None => return Err(2)
        };

        let (min, max) = match self.get_axis_min(name) {
            Some(min) => match self.get_axis_max(name) {
                Some(max) => (min, max),
                None => return Err(3)
            },
            None => return Err(3)
        };

        let mid: i64 = ((max - min)/2) as i64;
        let val = mid + (strength * (mid as f32)) as i64;

        match vjoy_rust::set_vjoystick_axis(self.get_vjoy_device_number(), hid, val) {
            Ok(_) => Ok(()),
            Err(_) => Err(4)
        }
    }

    fn verify_vjoystick_axis_compatibility(&self) -> Result<(), ()> {
        for (_, &(axis_index, _, _)) in self.get_axis_map() {
            if vjoy_rust::get_vjoystick_axis_exists(self.get_vjoy_device_number(), axis_index)== false {
                return Err(());
            }
        }

        Ok(())
    }
}

// HasJoysticks says that the implementor has at least one joystick, a two-dimensional analog input that is two axes
trait HasJoysticks: HasAxes {
    // Map of joystick names to (axis, axis) tuples
    fn get_joystick_map(&self) -> &HashMap<String, (String, String)>;

    fn get_joystick_axis_names(&self, name: &String) -> Option<&(String, String)> {
        match self.get_joystick_map().get(name) {
            Some(tuple) => Some(tuple),
            None => None
        }
    }

    // Set the joystick state, given a direction in degrees and a strength in the range [-1.0, 1.0]
    // Err(1): Unable to find joystick with given name
    // Err(2): Unable to set axis states
    fn set_joystick_state(&self, joystick: &String, direction: u16, strength: f32) -> Result<(), u8> {
        let (x, y): (&String, &String) = match self.get_joystick_axis_names(joystick) {
            Some(&(x, y)) => (x, y),
            None => return Err(1)
        };

        // Convert direction from degrees to radians
        let direction_rad: f32 = (direction as f32) * std::f32::consts::PI / 180.0;

        let x_strength = direction_rad.cos() * strength;
        let y_strength = direction_rad.sin() * strength;

        match self.set_axis_state(x, x_strength) {
            Ok(()) => (),
            Err(_) => return Err(2)
        }
        match self.set_axis_state(y, y_strength) {
            Ok(()) => (),
            Err(_) => return Err(2)
        }

        Ok(())
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

    fn verify_vjoystick_button_compatibility(&self) -> Result<(), ()>
    {
        if (vjoy_rust::get_vjoystick_button_count(self.get_vjoy_device_number()) as usize) < self.get_button_map().len() {
            return Err(());
        }

        Ok(())
    }
}

trait HasAxesAndButtons: HasAxes + HasButtons {
    fn verify_vjoystick_compatibility(&self) -> Result<(), ()> {
        match self.verify_vjoystick_axis_compatibility() {
            Ok(_) => (),
            Err(_) => return Err(())
        }

        match self.verify_vjoystick_button_compatibility() {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }
}

#[derive(Clone)]
pub enum Input {
    Joystick(String, u16, f32),
    Button(String, bool)
}
pub trait AcceptsInputs: HasJoysticks + HasButtons{
    fn set_input(&self, input: &Input) -> Result<(), &'static str> {
        match input.clone() {
            Input::Joystick(name, direction, strength) => self.set_joystick_state(&name, direction, strength),
            Input::Button(name, value) => self.set_button_state(&name, value)
        }
    }
}


//@todo track state
pub struct VN64C {
    axes: HashMap<String, (u32, i64, i64)>,
    joysticks: HashMap<String, (String, String)>,
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
impl HasJoysticks for VN64C {
    fn get_joystick_map(&self) -> &HashMap<String, (String, String)> {
        return self.joysticks;
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
impl HasAxesAndButtons for VN64C {}
impl AcceptsInputs for VN64C {}

impl VN64C {
    // Err(1): unable to get N64 hardware
    // Err(2): vjoystick doesn't meet N64 controller requirements
    // Err(3): unable to claim and reset vjoystick
    pub fn new(vjoy_device_number: u32) -> Result<Self, u8> {
        let (axes, joysticks, buttons) = match get_n64_controller_hardware(vjoy_device_number) {
            Ok((axes, joysticks, buttons)) => (axes, buttons),
            Err(_) => return Err(1)
        };

        let vn64c = VN64C { axes: axes, buttons: buttons, vjoy_device_number: vjoy_device_number };

        match vn64c.verify_vjoystick_compatibility() {
            Ok(_) => (),
            Err(_) => return Err(2)
        }

        match vn64c.claim_and_reset() {
            Ok(_) => Ok(vn64c),
            Err(_) => Err(3)
        }
    }
}

fn get_n64_controller_hardware(vjoy_device_number: u32) -> Result<(HashMap<String, (u32, i64, i64)>, HashMap<String, (String, String)>,HashMap<String, u8>), u8> {
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


//@todo track state
pub struct VGcnC {
    axes: HashMap<String, (u32, i64, i64)>,
    buttons: HashMap<String, u8>,
    vjoy_device_number: u32
}

impl IsVJoyDevice for VGcnC {
    fn get_vjoy_device_number(&self) -> u32 {
        return self.vjoy_device_number;
    }
}

impl HasAxes for VGcnC {
    fn get_axis_map(&self) -> &HashMap<String, (u32, i64, i64)> {
        return &self.axes;
    }

    fn get_axis_state(&self, name: &String) -> Option<i64> {
        return Some(0); //@todo
    }
}

impl HasButtons for VGcnC {
    fn get_button_map(&self) -> &HashMap<String, u8> {
        return &self.buttons;
    }

    fn get_button_state(&self, name: &String) -> Option<bool> {
        return Some(false);
    }
}

impl HasAxesAndButtons for VGcnC {}

impl AcceptsInputs for VGcnC {}

impl VGcnC {
    // Err(1): unable to get GCN hardware
    // Err(2): vjoystick doesn't meet GCN controller requirements
    // Err(3): unable to claim and reset vjoystick
    pub fn new(vjoy_device_number: u32) -> Result<Self, u8> {
        let (axes, buttons) = match get_gcn_controller_hardware(vjoy_device_number) {
            Ok((axes, buttons)) => (axes, buttons),
            Err(_) => return Err(1)
        };

        let vgcnc = VGcnC { axes: axes, buttons: buttons, vjoy_device_number: vjoy_device_number };

        match vgcnc.verify_vjoystick_compatibility() {
            Ok(_) => (),
            Err(_) => return Err(2)
        }

        match vgcnc.claim_and_reset() {
            Ok(_) => Ok(vgcnc),
            Err(_) => Err(3)
        }
    }
}

fn get_gcn_controller_hardware(vjoy_device_number: u32)
        -> Result<(HashMap<String, (u32, i64, i64)>, HashMap<String, u8>), u8>
{
    let mut axes = HashMap::new();

    let jx_min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, 0x30) {
        Ok(min) => min,
        Err(msg) => return Err(1)
    };
    let jx_max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, 0x30) {
        Ok(max) => max,
        Err(_) => return Err(2)
    };
    let jy_min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, 0x31) {
        Ok(min) => min,
        Err(msg) => return Err(3)
    };
    let jy_max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, 0x31) {
        Ok(max) => max,
        Err(_) => return Err(4)
    };

    let cx_min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, 0x33) {
        Ok(min) => min,
        Err(msg) => return Err(1)
    };
    let cx_max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, 0x33) {
        Ok(max) => max,
        Err(_) => return Err(2)
    };
    let cy_min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, 0x34) {
        Ok(min) => min,
        Err(msg) => return Err(3)
    };
    let cy_max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, 0x34) {
        Ok(max) => max,
        Err(_) => return Err(4)
    };
    
    axes.insert(String::from("jx"), (0x30, jx_min, jx_max));
    axes.insert(String::from("jy"), (0x31, jy_min, jy_max));
    axes.insert(String::from("cx"), (0x33, cx_min, cx_max));
    axes.insert(String::from("cy"), (0x34, cy_min, cy_max));

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

    Ok((axes, buttons))
}