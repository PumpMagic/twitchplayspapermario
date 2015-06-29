#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
mod vjoyinterface;
extern crate std;
extern crate libc;

// Hardcode the vJoy virtual joystick index of whatever joystick our emulator listens to
const VJOY_DEVICE_NUMBER: libc::c_uint = 1;

// Number of digital inputs on a standard N64 controller
const NUM_N64_BUTTONS: u8 = 14;

// Control constants - what magic numbers do we need to play with each virtual device input?
// USB HID joystick constants. We assume our emulator is configured to map virtual joystick to N64 joystick
const HID_JOYSTICK_X: libc::c_uint = 0x30;
const HID_JOYSTICK_Y: libc::c_uint = 0x31;

// Default emulator button bindings. vJoy is one-based with buttons.
const BUTTON_A: libc::c_uchar = 0x01;
const BUTTON_B: libc::c_uchar = 0x02;
const BUTTON_Z: libc::c_uchar = 0x03;
const BUTTON_L: libc::c_uchar = 0x04;
const BUTTON_R: libc::c_uchar = 0x05;
const BUTTON_START: libc::c_uchar = 0x06;
const BUTTON_CUP: libc::c_uchar = 0x07;
const BUTTON_CDOWN: libc::c_uchar = 0x08;
const BUTTON_CLEFT: libc::c_uchar = 0x09;
const BUTTON_CRIGHT: libc::c_uchar = 0x10;
const BUTTON_DUP: libc::c_uchar = 0x11;
const BUTTON_DDOWN: libc::c_uchar = 0x12;
const BUTTON_DLEFT: libc::c_uchar = 0x13;
const BUTTON_DRIGHT: libc::c_uchar = 0x14;

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

// Info about our virtual joystick
// Post-initialization, this should all be read-only
pub struct VirtualN64ControllerProps {
    pub vjoy_device_number: libc::c_uint,
    
    pub x_min: libc::c_long,
    pub x_max: libc::c_long,
    pub y_min: libc::c_long,
    pub y_max: libc::c_long,
}

// Last-set values
pub struct VirtualN64ControllerState {
    pub x: libc::c_long,
    pub y: libc::c_long,
    pub a: libc::c_int,
    pub b: libc::c_int,
    pub z: libc::c_int,
    pub l: libc::c_int,
    pub r: libc::c_int,
    pub cup: libc::c_int,
    pub cdown: libc::c_int,
    pub cleft: libc::c_int,
    pub cright: libc::c_int,
    pub dup: libc::c_int,
    pub ddown: libc::c_int,
    pub dleft: libc::c_int,
    pub dright: libc::c_int,
}

pub struct VirtualN64Controller {
    pub props: VirtualN64ControllerProps,
    pub state: VirtualN64ControllerState,
}

impl Default for VirtualN64Controller {
    fn default() -> VirtualN64Controller {
        VirtualN64Controller {
            props: VirtualN64ControllerProps {
                vjoy_device_number: 0,
                x_min: 0,
                x_max: 0,
                y_min: 0,
                y_max: 0,
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
                dright: 0,
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
    
    return true;
}

fn does_vjoystick_axis_exist(index: libc::c_uint, axis: libc::c_uint) -> bool {
    unsafe {
        let axis_exists = vjoyinterface::GetVJDAxisExist(index, axis);
        if axis_exists == 0 {
            println!("vJoy device {} doesn't have axis {}", index, axis);
            return false;
        }
    }
    
    return true;
}

//@todo technically should be an Optional, but the button count function never fails sooooooooooo this shouldn't riiiiiiight?
fn get_vjoystick_axis_min(index: libc::c_uint, axis: libc::c_uint) -> libc::c_long {
    unsafe {    
        let mut min: libc::c_long = 0;
        let min_raw_pointer = &mut min as *mut libc::c_long;
        let min_result = vjoyinterface::GetVJDAxisMin(index, axis, min_raw_pointer);
        if min_result == 0 {
            //@todo return option.absent
        }
        
        println!("vJoy device {} axis {} min is {}", index, axis, min);
        return min;
    }
}

//@todo technically should be an Optional, but the button count function never fails sooooooooooo this shouldn't riiiiiiight?
fn get_vjoystick_axis_max(index: libc::c_uint, axis: libc::c_uint) -> libc::c_long {
    unsafe {    
        let mut max: libc::c_long = 0;
        let max_raw_pointer = &mut max as *mut libc::c_long;
        let max_result = vjoyinterface::GetVJDAxisMax(index, axis, max_raw_pointer);
        if max_result == 0 {
            //@todo return option.absent
        }
        
        println!("vJoy device {} axis {} max is {}", index, axis, max);
        return max;
    }
}

fn get_vjoystick_button_count(index: libc::c_uint) -> u8 {
    unsafe {
        let num_buttons = vjoyinterface::GetVJDButtonNumber(index);
        println!("vJoy device {} has {} buttons", index, num_buttons);
        return num_buttons as u8;
    }
}

fn get_vjoystick_status(index: libc::c_uint) -> vjoyinterface::Enum_VjdStat {
    unsafe {
        let joystick_status = vjoyinterface::GetVJDStatus(index);
        println!("vJoy device {} has status {}", index, joystick_status);
        return joystick_status;
    }
}

fn claim_vjoystick(index: libc::c_uint) -> bool {
    unsafe {
        let joystick_status = get_vjoystick_status(index);
        if joystick_status == vjoyinterface::VJD_STAT_FREE {
            // Try to claim it
            let acquire_vjd_result = vjoyinterface::AcquireVJD(index);
            if acquire_vjd_result == 0 {
                return false;
            } else {
                return true;
            }
        } else if joystick_status == vjoyinterface::VJD_STAT_OWN {
            // We've already claimed it
            return true;
        }
    }
    
    return false;
}

fn reset_vjoystick(index: libc::c_uint) -> bool {
    unsafe {
        let reset_result = vjoyinterface::ResetVJD(index);
        if reset_result == 0 {
            return false;
        }
    }
    
    return true;
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

fn populate_vn64c(controller: &mut VirtualN64Controller) -> bool {
    // Get and capture vjoystick min and max, set default joystick and button values
    controller.props.x_min = get_vjoystick_axis_min(controller.props.vjoy_device_number, HID_JOYSTICK_X);
    controller.props.x_max = get_vjoystick_axis_max(controller.props.vjoy_device_number, HID_JOYSTICK_X);
    controller.props.y_min = get_vjoystick_axis_min(controller.props.vjoy_device_number, HID_JOYSTICK_Y);
    controller.props.y_max = get_vjoystick_axis_max(controller.props.vjoy_device_number, HID_JOYSTICK_Y);
    
    controller.state.x = (controller.props.x_max - controller.props.x_min) / 2;
    controller.state.y = (controller.props.y_max - controller.props.y_min) / 2;
    
    println!("Device number: {}", controller.props.vjoy_device_number);
    println!("\tX min: {}", controller.props.x_min);
    println!("\tX max: {}", controller.props.x_max);
    println!("\tY min: {}", controller.props.y_min);
    println!("\tY max: {}", controller.props.y_max);
    println!("\tX: {}", controller.state.x);
    println!("\tY: {}", controller.state.y);
    
    return true;
}

// Exposed functions
// Initialize a virtual N64 controller
// After calling this successfully, controller is a valid handle for other functions
pub fn init() -> Result<VirtualN64Controller, &'static str> {
    let mut controller = VirtualN64Controller { ..Default::default() };
    
    controller.props.vjoy_device_number = VJOY_DEVICE_NUMBER;
    
    if !is_vjoy_enabled() { return Err("vJoy isn't enabled"); }
    if !is_vjoystick_n64(controller.props.vjoy_device_number) { return Err("Virtual joystick is not capable of emulating an N64 controller"); }
    if !claim_vjoystick(controller.props.vjoy_device_number) { return Err("Unable to claim virtual joystick"); }
    if !reset_vjoystick(controller.props.vjoy_device_number) { return Err("Unable to reset virtual joystick"); }
    
    if !populate_vn64c(&mut controller) { return Err("Unable to populate virtual N64 controller"); }
    
    Ok(controller)
    
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
    //@todo better way to map these?
    let vjoy_button = match button {
        VirtualN64ControllerButton::A => BUTTON_A,
        VirtualN64ControllerButton::B => BUTTON_B,
        VirtualN64ControllerButton::Z => BUTTON_Z,
        VirtualN64ControllerButton::L => BUTTON_L,
        VirtualN64ControllerButton::R => BUTTON_R,
        VirtualN64ControllerButton::Start => BUTTON_START,
        VirtualN64ControllerButton::Cup => BUTTON_CUP,
        VirtualN64ControllerButton::Cdown => BUTTON_CDOWN,
        VirtualN64ControllerButton::Cleft => BUTTON_CLEFT,
        VirtualN64ControllerButton::Cright => BUTTON_CRIGHT,
        VirtualN64ControllerButton::Dup => BUTTON_DUP,
        VirtualN64ControllerButton::Ddown => BUTTON_DDOWN,
        VirtualN64ControllerButton::Dleft => BUTTON_DLEFT,
        VirtualN64ControllerButton::Dright => BUTTON_DRIGHT
    };
    
    if !(set_vjoystick_button(controller.props.vjoy_device_number, vjoy_button, value as libc::c_int)) {
        println!("Setting button failed!");
        return false;
    }
    
    println!("Set button {:?} to {}", button, value as libc::c_int);
    
    return true;
}