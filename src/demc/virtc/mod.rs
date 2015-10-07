#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
pub mod vjoy_rust;

extern crate std;

use std::collections::HashMap;


// IsVJoyDevice says that the implementor is a representation of a vJoy virtual joystick
pub trait IsVJoyDevice {
    fn get_device_number(&self) -> u32;

    // Convenience function for claiming and resetting the vJoy device with the given number
    // Err(1): vJoy isn't enabled
    // Err(2): Unable to claim vjoystick
    // Err(3): Unable to reset vjoystick
    fn claim_and_reset(&self) -> Result<(), u8> {
        if vjoy_rust::is_vjoy_enabled() == false {
            return Err(1);
        }

        match vjoy_rust::claim_vjoystick(self.get_device_number()) {
            Err(msg) => return Err(2),
            _ => ()
        }

        match vjoy_rust::reset_vjoystick(self.get_device_number()) {
            Err(_) => return Err(3),
            _ => Ok(())
        }
    }
}

// HasAxes says that the implementor contains at least one vJoy virtual axis
//@todo separate mins and maxes into their own maps and initialize those locally so that user libs don't need to
pub trait HasAxes: IsVJoyDevice {
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
    //@todo
    fn get_axis_state(&self, name: &String) -> Option<i64> {
        return Some(0);
    }

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

        match vjoy_rust::set_vjoystick_axis(self.get_device_number(), hid, val) {
            Ok(_) => Ok(()),
            Err(_) => Err(4)
        }
    }

    fn verify_vjoystick_axis_compatibility(&self) -> Result<(), ()> {
        for (_, &(axis_index, _, _)) in self.get_axis_map() {
            if vjoy_rust::get_vjoystick_axis_exists(self.get_device_number(), axis_index)== false {
                return Err(());
            }
        }

        Ok(())
    }
}

// HasJoysticks says that the implementor has at least one joystick, a two-dimensional analog input that is two axes
pub trait HasJoysticks: HasAxes {
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
        let (x, y) = match self.get_joystick_axis_names(joystick) {
            Some(&(ref x, ref y)) => (x, y),
            None => return Err(1)
        };

        // Convert direction from degrees to radians
        let direction_rad: f32 = (direction as f32) * std::f32::consts::PI / 180.0;

        let x_strength = direction_rad.cos() * strength;
        let y_strength = direction_rad.sin() * strength;

        match self.set_axis_state(&x, x_strength) {
            Ok(()) => (),
            Err(_) => return Err(2)
        }
        match self.set_axis_state(&y, y_strength) {
            Ok(()) => (),
            Err(_) => return Err(2)
        }

        Ok(())
    }
}

pub trait HasButtons: IsVJoyDevice {
    fn get_button_map(&self) -> &HashMap<String, u8>;

    fn get_button_index(&self, name: &String) -> Option<u8> {
        match self.get_button_map().get(name) {
            Some(index) => Some(*index),
            None => None
        }
    }
    
    //@todo
    fn get_button_state(&self, name: &String) -> Option<bool> {
        return Some(false);
    }

    // Err(1): Unable to set virtual joystick button
    fn set_button_state(&self, name: &String, value: bool) -> Result<(), u8> {
        //@todo unwrap here
        let index = self.get_button_index(name).unwrap();

        let valc = value as i32;

        match vjoy_rust::set_vjoystick_button(self.get_device_number(), index, valc) {
            Ok(_) => Ok(()),
            Err(_) => Err(1)
        }
    }

    fn verify_vjoystick_button_compatibility(&self) -> Result<(), ()> {
        if (vjoy_rust::get_vjoystick_button_count(self.get_device_number()) as usize) < self.get_button_map().len() {
            return Err(());
        }
        
        Ok(())
    }
}

pub trait HasAxesAndButtons: HasAxes + HasButtons {
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
    fn set_input(&self, input: &Input) -> Result<(), u8> {
        match input.clone() {
            Input::Joystick(name, direction, strength) => self.set_joystick_state(&name, direction, strength),
            Input::Button(name, value) => self.set_button_state(&name, value)
        }
    }
}