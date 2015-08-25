#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
mod vjoy_rust;

extern crate std;


struct ControllerHardware {
    axes: HashMap<String, u32>,
    buttons: HashMap<String, u32>
}

pub fn get_n64_controller_hardware() -> ControllerHardware {
    let mut axes = HashMap::new();
    axes.insert("x", 0x30); // USB HID
    axes.insert("y", 0x31); // USB HID

    let mut buttons = HashMap::new();
    buttons.insert("a", 0x01);
    buttons.insert("b", 0x02);
    buttons.insert("z", 0x03);
    buttons.insert("l", 0x04);
    buttons.insert("r", 0x05);
    buttons.insert("start", 0x06);
    buttons.insert("cup", 0x07);
    buttons.insert("cdown", 0x08);
    buttons.insert("cleft", 0x09);
    buttons.insert("cright", 0x0a);
    buttons.insert("dup", 0x0b);
    buttons.insert("ddown", 0x0c);
    buttons.insert("dleft", 0x0d);
    buttons.insert("dright", 0x0e);

    ControllerHardware { axes: axes, buttons: buttons }
}

pub fn get_gcn_controller_hardware() -> ControllerHardware {
    let mut axes = HashMap::new();
    axes.insert("jx", 0x30); // USB HID
    axes.insert("jy", 0x31); // USB HID
    axes.insert("cx", 0x32); // USB HID???
    axes.insert("cy", 0x33); // USB HID???

    let mut buttons = HashMap::new();
    buttons.insert("a", 0x01);
    buttons.insert("b", 0x02);
    buttons.insert("x", 0x03);
    buttons.insert("y", 0x04);
    buttons.insert("z", 0x05);
    buttons.insert("l", 0x06);
    buttons.insert("r", 0x07);
    buttons.insert("dup", 0x08);
    buttons.insert("ddown", 0x09);
    buttons.insert("dleft", 0x0a);
    buttons.insert("dright", 0x0b);

    ControllerHardware { axes: axes, buttons: buttons }
}

// Properties (non-input-state characteristics) about a virtual N64 controller
// Post-initialization, this should all be read-only
struct Props {
    vjoy_device_number: u32,

    hardware: ControllerHardware,

    axis_mins: HashMap<u32, i64>,
    axis_maxes: HashMap<u32, i64>,
}

// Comprehensive status of a virtual N64 controller's inputs
struct State {
    axes: HashMap<u32, i64>,
    buttons: HashMap<u32, i32>
}

pub struct Controller {
    props: Props,
    state: State
}

#[derive(Clone, Copy)]
pub enum InputCommand {
    // direction is in degrees - strength is a number between 0 and 1
    //@todo this module shouldn't be the one converting between direction+str and x+y
    //Joystick { direction: u16, strength: f32 },
    Axis { index: u32, value: i64 },
    Button { index: u32, value: bool }
}

impl InputCommand {
    //@todo actually do something, once this module doesn't convert from dir+strength
    fn is_compatible_with(&self, ch: &ControllerHardware) -> bool {
        match *self {
            InputCommand::Joystick { direction, strength } => {
                if direction > 359 {
                    return false;
                } else if strength < 0.0 || strength > 1.0 {
                    return false;
                }
                return true
            },
            InputCommand::Button { name: _, value: _ } => { return true; }
        }
    }
}

impl Controller {
    // Verify that a vJoy device can act as a virtual N64 controller, and if so, return a virtual
    // N64 controller
    pub fn new(vjoy_device_number: u32, hardware: ControllerHardware) -> Result<Controller, String> {
        let vjoy_device_number_native = vjoy_device_number;

        match vjoy_rust::get_vjoy_is_enabled() {
            Ok(val) => {
                if val == false {
                    return Err(format!("vJoy isn't enabled. Have you installed vJoy?"));
                }
            },
            Err(err) => return Err(format!("Unable to check if vJoy is enabled. Have you installed vJoy?"))
        }

        match verify_vjoystick_hardware(vjoy_device_number_native, &hardware) {
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
            Err(msg) => return Err(format!("{}", msg)),
            _ => ()
        }

        match Controller::from_hardware(vjoy_device_number_native, hardware) {
            Err(msg) => Err(format!("{}", msg)),
            Ok(controller) => Ok(controller)
        }
    }

    // Make and initialize a virtual N64 controller struct given the device number
    fn from_hardware(vjoy_device_number: u32, hardware: ControllerHardware) -> Result<Controller, &'static str> {
        let mut props = Props {
            vjoy_device_number: vjoy_device_number,

            hardware: hardware,

            axis_mins: HashMap::new(),
            axis_maxes: HashMap::new()
        };
        let mut state = State {
            axes: HashMap::new(),
            buttons: HashMap::new()
        };

        // Get and capture vjoystick min and max; set joystick values to neutral
        for (_, hid) in props.hardware.axes.iter() {
            let min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, hid) {
                Ok(min) => min,
                Err(msg) => return Err(msg)
            };
            let max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, hid) {
                Ok(max) => max,
                Err(msg) => return Err(msg)
            };

            props.axis_mins.insert(hid, min);
            props.axis_maxes.insert(hid, max);
            state.axes.insert(hid, (max-min)/2);
        }

        for (_, index) in props.hardware.buttons.iter() {
            state.buttons.insert(index, 0);
        }

        Ok(Controller { props: props, state: state })
    }
    
    pub fn change_input(&self, command: &InputCommand) -> Result<(), &'static str> {
        match command.is_valid() {
            true => (),
            false => { return Err("Input command is invalid. Valid directions: [0, 359]; valid strengths: [0.0, 1.0]"); }
        }
        
        match *command {
            InputCommand::Joystick { direction, strength } => {
                self.change_joystick(direction, strength)
            },
            InputCommand::Button { name, value } => {
                self.change_button(name, value)
            }
        }
    }
    
    fn change_joystick(&self, direction: u16, strength: f32) -> Result<(), &'static str> {
        // Convert direction from degrees to radians
        let direction_rad: f32 = (direction as f32) * std::f32::consts::PI / 180.0;

        let x_strength = direction_rad.cos() * strength;
        let y_strength = direction_rad.sin() * strength;

        let x_mid: i64 = ((self.props.x_max - self.props.x_min)/2) as i64;
        let y_mid: i64 = ((self.props.y_max - self.props.y_min)/2) as i64;

        let x = x_mid + (x_strength * (x_mid as f32)) as i64;
        let y = y_mid + (y_strength * (y_mid as f32)) as i64;

        match vjoy_rust::set_vjoystick_axis(self.props.vjoy_device_number, HID_JOYSTICK_X, x) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set X axis")
        }
        //self.state.x = x;

        match vjoy_rust::set_vjoystick_axis(self.props.vjoy_device_number, HID_JOYSTICK_Y, y) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set Y axis")
        }
        //self.state.y = y;
        
        Ok(())
    }
    
    fn change_button(&self, name: ButtonName, value: bool) -> Result<(), &'static str> {
        let valc = value as i32;

        match vjoy_rust::set_vjoystick_button(self.props.vjoy_device_number, name.get_vjoy_button_index(), valc) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set virtual joystick button")
        }

        /*
        match button {
            ButtonName::A => self.state.a = valc,
            ButtonName::B => self.state.b = valc,
            ButtonName::Z => self.state.z = valc,
            ButtonName::L => self.state.l = valc,
            ButtonName::R => self.state.r = valc,
            ButtonName::Start => self.state.start = valc,
            ButtonName::Cup => self.state.cup = valc,
            ButtonName::Cdown => self.state.cdown = valc,
            ButtonName::Cleft => self.state.cleft = valc,
            ButtonName::Cright => self.state.cright = valc,
            ButtonName::Dup => self.state.dup = valc,
            ButtonName::Ddown => self.state.ddown = valc,
            ButtonName::Dleft => self.state.dleft = valc,
            ButtonName::Dright => self.state.dright = valc
        }
        */
        
        Ok(())
    }
        
    pub fn write_to_console(&self) {
        println!("X: {} Y: {} A: {} B: {} Z: {} L: {} R: {} S: {}", self.state.x, self.state.y, self.state.a, self.state.b, self.state.z, self.state.l, self.state.r, self.state.start);
        //println!("CU: {} CD: {} CL: {} CR: {}", self.state.cup, self.state.cdown, self.state.cleft, self.state.cright);
        //println!("DU: {} DD: {} DL: {} DR: {}", self.state.dup, self.state.ddown, self.state.dleft, self.state.dright);
    }
}


//@todo implement
fn verify_vjoystick_hardware(index: u32, hardware: &ControllerHardware) -> Result<(), String> {
    match vjoy_rust::get_vjoystick_axis_exists(index, HID_JOYSTICK_X) {
        Ok(exists) => {
            if exists == false {
                return Err(format!("No X axis"));
            }
        },
        Err(()) => return Err(format!("Unable to check for X axis"))
    }

    match vjoy_rust::get_vjoystick_axis_exists(index, HID_JOYSTICK_Y) {
        Ok(exists) => {
            if exists == false {
                return Err(format!("No Y axis"));
            }
        },
        Err(()) => return Err(format!("Unable to check for Y axis"))
    }

    match vjoy_rust::get_vjoystick_button_count(index) {
        Ok(buttons) => {
            if buttons < 14 {
                return Err(format!("Less than {} buttons", 14));
            }
        },
        Err(()) => return Err(format!("Unable to get button count"))
    }

    Ok(())
}
