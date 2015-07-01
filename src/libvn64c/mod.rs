#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
mod vjoyinterface;
extern crate std;
extern crate libc;

// Number of digital inputs on a standard N64 controller
const NUM_N64_BUTTONS: u8 = 14;

// Control constants - what magic numbers do we need to play with each virtual device input?
// USB HID joystick constants. We assume our emulator is configured to map virtual joystick to N64 joystick
const HID_JOYSTICK_X: libc::c_uint = 0x30;
const HID_JOYSTICK_Y: libc::c_uint = 0x31;

#[derive(Debug)]
pub enum VirtualN64ControllerButton {
    A,
    B,
    Z,
    L,
    R,
    Start,
    Cup,
    Cdown,
    Cleft,
    Cright,
    Dup,
    Ddown,
    Dleft,
    Dright
}

// Default emulator button bindings. vJoy button indices are one-based
impl VirtualN64ControllerButton {
    fn get_vjoy_button_index(&self) -> libc::c_uchar {
        match *self {
            VirtualN64ControllerButton::A => 0x01,
            VirtualN64ControllerButton::B => 0x02,
            VirtualN64ControllerButton::Z => 0x03,
            VirtualN64ControllerButton::L => 0x04,
            VirtualN64ControllerButton::R => 0x05,
            VirtualN64ControllerButton::Start => 0x06,
            VirtualN64ControllerButton::Cup => 0x07,
            VirtualN64ControllerButton::Cdown => 0x08,
            VirtualN64ControllerButton::Cleft => 0x09,
            VirtualN64ControllerButton::Cright => 0x10,
            VirtualN64ControllerButton::Dup => 0x11,
            VirtualN64ControllerButton::Ddown => 0x12,
            VirtualN64ControllerButton::Dleft => 0x13,
            VirtualN64ControllerButton::Dright => 0x14
        }
    }
}

// Info about our virtual joystick
// Post-initialization, this should all be read-only
struct VirtualN64ControllerProps {
    vjoy_device_number: libc::c_uint,
    
    x_min: libc::c_long,
    x_max: libc::c_long,
    y_min: libc::c_long,
    y_max: libc::c_long,
}

// Last-set values
struct VirtualN64ControllerState {
    x: libc::c_long,
    y: libc::c_long,
    a: libc::c_int,
    b: libc::c_int,
    z: libc::c_int,
    l: libc::c_int,
    r: libc::c_int,
    cup: libc::c_int,
    cdown: libc::c_int,
    cleft: libc::c_int,
    cright: libc::c_int,
    dup: libc::c_int,
    ddown: libc::c_int,
    dleft: libc::c_int,
    dright: libc::c_int,
}

pub struct VirtualN64Controller {
    props: VirtualN64ControllerProps,
    state: VirtualN64ControllerState,
}

impl Default for VirtualN64Controller {
    fn default() -> VirtualN64Controller {
        VirtualN64Controller {
            props: VirtualN64ControllerProps {
                vjoy_device_number: 0,
                x_min: 0,
                x_max: 0,
                y_min: 0,
                y_max: 0
            },
            state: VirtualN64ControllerState {
                x: 0,
                y: 0,
                a: 0,
                b: 0,
                z: 0,
                l: 0,
                r: 0,
                cup: 0,
                cdown: 0,
                cleft: 0,
                cright: 0,
                dup: 0,
                ddown: 0,
                dleft: 0,
                dright: 0
            }
        }
    }
}

// vJoy device utility functions
fn is_vjoy_enabled() -> bool {
    unsafe {
        let vjoy_enabled = vjoyinterface::vJoyEnabled();
        if vjoy_enabled == 0 {
            println!("vJoy isn't enabled");
            return false;
        }
    }
    
    true
}

fn does_vjoystick_axis_exist(index: libc::c_uint, axis: libc::c_uint) -> bool {
    unsafe {
        let axis_exists = vjoyinterface::GetVJDAxisExist(index, axis);
        if axis_exists == 0 {
            println!("vJoy device {} doesn't have axis {}", index, axis);
            return false;
        }
    }
    
    true
}

fn get_vjoystick_axis_min(index: libc::c_uint, axis: libc::c_uint) -> Result<libc::c_long, &'static str> {
    unsafe {    
        let mut min: libc::c_long = 0;
        let min_raw_pointer = &mut min as *mut libc::c_long;
        let min_result = vjoyinterface::GetVJDAxisMin(index, axis, min_raw_pointer);
        if min_result == 0 {
            return Err("Unable to get axis minimum");
        }
        
        Ok(min)
    }
}

fn get_vjoystick_axis_max(index: libc::c_uint, axis: libc::c_uint) -> Result<libc::c_long, &'static str> {
    unsafe {    
        let mut max: libc::c_long = 0;
        let max_raw_pointer = &mut max as *mut libc::c_long;
        let max_result = vjoyinterface::GetVJDAxisMax(index, axis, max_raw_pointer);
        if max_result == 0 {
            return Err("Unable to get axis maximum");
        }
        
        Ok(max)
    }
}

fn get_vjoystick_button_count(index: libc::c_uint) -> u8 {
    unsafe {
        let num_buttons = vjoyinterface::GetVJDButtonNumber(index);
        println!("vJoy device {} has {} buttons", index, num_buttons);
        num_buttons as u8
    }
}

fn get_vjoystick_status(index: libc::c_uint) -> vjoyinterface::Enum_VjdStat {
    unsafe {
        let joystick_status = vjoyinterface::GetVJDStatus(index);
        println!("vJoy device {} has status {}", index, joystick_status);
        return joystick_status;
    }
}

fn claim_vjoystick(index: libc::c_uint) -> Result<(), &'static str> {
    unsafe {
        let joystick_status = get_vjoystick_status(index);
        if joystick_status == vjoyinterface::VJD_STAT_FREE {
            // Try to claim it
            let acquire_vjd_result = vjoyinterface::AcquireVJD(index);
            if acquire_vjd_result == 0 {
                return Err("Virtual joystick is available, but unable to acquire it");
            } else {
                return Ok(());
            }
        } else if joystick_status == vjoyinterface::VJD_STAT_OWN {
            // We've already claimed it
            return Ok(());
        }
    }
    
    return Err("Virtual joystick is owned by someone else, missing, or in unknown state");
}

fn reset_vjoystick(index: libc::c_uint) -> Result<(), &'static str> {
    unsafe {
        let reset_result = vjoyinterface::ResetVJD(index);
        if reset_result == 0 {
            return Err("vJoy reset function returned failure");
        }
    }
    
    return Ok(());
}

fn set_vjoystick_axis(index: libc::c_uint, axis: libc::c_uint, value: libc::c_long) -> bool {
    unsafe {
        let set_x_result = vjoyinterface::SetAxis(value, index, axis);
        if set_x_result == 0 {
            return false;
        }
    }
    
    return true;
}

fn set_vjoystick_button(index: libc::c_uint, button: libc::c_uchar, value: libc::c_int) -> bool {
    unsafe {
        let set_result = vjoyinterface::SetBtn(value, index, button);
        if set_result == 0 {
            return false;
        }
    }
    
    return true;
}

// Test if a vJoy device has the controls we need to treat it like an N64 controller
// If this fails, the vJoy device should be configured manually using vJoy's supplied configuration tool
fn is_vjoystick_n64(index: libc::c_uint) -> bool {
    if !does_vjoystick_axis_exist(index, HID_JOYSTICK_X) { return false; }
    if !does_vjoystick_axis_exist(index, HID_JOYSTICK_Y) { return false; }
    if !(get_vjoystick_button_count(index) >= NUM_N64_BUTTONS) { return false; }
    
    return true;
}

// Make a virtual N64 controller given a vJoy device number
fn make_vn64c(vjoy_device_number: libc::c_uint) -> Result<(VirtualN64Controller), &'static str> {
    let mut controller = VirtualN64Controller { ..Default::default() };
    
    controller.props.vjoy_device_number = vjoy_device_number;
    
    // Get and capture vjoystick min and max
    match get_vjoystick_axis_min(vjoy_device_number, HID_JOYSTICK_X) {
        Ok(min) => controller.props.x_min = min,
        Err(msg) => return Err(msg)
    };
    match get_vjoystick_axis_max(vjoy_device_number, HID_JOYSTICK_X) {
        Ok(max) => controller.props.x_max = max,
        Err(msg) => return Err(msg)
    };
    match get_vjoystick_axis_min(vjoy_device_number, HID_JOYSTICK_Y) {
        Ok(min) => controller.props.y_min = min,
        Err(msg) => return Err(msg)
    };
    match get_vjoystick_axis_min(vjoy_device_number, HID_JOYSTICK_Y) {
        Ok(max) => controller.props.y_max = max,
        Err(msg) => return Err(msg)
    };
    
    // Set joystick values to neutral
    controller.state.x = (controller.props.x_max - controller.props.x_min) / 2;
    controller.state.y = (controller.props.y_max - controller.props.y_min) / 2;
    
    Ok(controller)
}

// Exposed functions
// Initialize a virtual N64 controller
// After calling this successfully, controller is a valid handle for other functions
pub fn init(vjoy_device_number: u8) -> Result<VirtualN64Controller, &'static str> {
    let vjoy_device_number_native = vjoy_device_number as libc::c_uint;
    
    if !is_vjoy_enabled() { return Err("vJoy isn't enabled"); }
    if !is_vjoystick_n64(vjoy_device_number_native) { return Err("Virtual joystick is not capable of emulating an N64 controller"); }
    match claim_vjoystick(vjoy_device_number_native) {
        Err(msg) => return Err(msg),
        _ => ()
    };
    match reset_vjoystick(vjoy_device_number_native) {
        Err(msg) => return Err(msg),
        _ => ()
    };
    match make_vn64c(vjoy_device_number_native) {
        Err(msg) => Err(msg),
        Ok(controller) => Ok(controller)
    }
}

// Set the virtual joystick, assuming that direction is in degrees and strength is a number 0-1 inclusive
//@todo don't trust controller argument
//@todo validate arguments (direction: 0-359, strength: 0-1)
pub fn set_joystick(controller: &mut VirtualN64Controller, direction: u16, strength: f32) -> Result<(), &'static str> {
    // Convert direction from degrees to radians
    let direction_rad: f32 = (direction as f32) * std::f32::consts::PI / 180.0;
    
    let x_strength = direction_rad.cos() * strength;
    let y_strength = direction_rad.sin() * strength;
    
    let x_mid: libc::c_long = ((controller.props.x_max - controller.props.x_min)/2) as libc::c_long;
    let y_mid: libc::c_long = ((controller.props.y_max - controller.props.y_min)/2) as libc::c_long;
    
    let x = x_mid + (x_strength * (x_mid as f32)) as libc::c_long;
    let y = y_mid + (y_strength * (y_mid as f32)) as libc::c_long;
    
    controller.state.x = x;
    controller.state.y = y;
    
    if !set_vjoystick_axis(controller.props.vjoy_device_number, HID_JOYSTICK_X, controller.state.x) {
        return Err("Unable to set X axis");
    }
    if !set_vjoystick_axis(controller.props.vjoy_device_number, HID_JOYSTICK_Y, controller.state.y) {
        return Err("Unable to set Y axis");
    }
    
    println!("Set joystick: x = {} y = {}", x_strength, y_strength);
    println!("Scaled for power, that's x = {} y = {}", x, y);
    
    Ok(())
}

pub fn set_button(controller: &VirtualN64Controller, button: VirtualN64ControllerButton, value: bool) -> bool {
    if !(set_vjoystick_button(controller.props.vjoy_device_number, button.get_vjoy_button_index(), value as libc::c_int)) {
        println!("Setting button failed!");
        return false;
    }
    
    println!("Set button {:?} to {}", button, value as libc::c_int);
    
    return true;
}