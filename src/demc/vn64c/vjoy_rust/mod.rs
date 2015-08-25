mod vjoyinterface;

extern crate libc;

// Rustifying vJoy wrapper functions + convenience functions

// Wrapper functions

pub fn get_vjoy_is_enabled() -> Result<bool, ()> {
    unsafe {
        let vjoy_enabled = vjoyinterface::vJoyEnabled();
        if vjoy_enabled == 0 {
                Ok(false)
        } else {
                Ok(true)
        }
    }
}

pub fn get_vjoystick_axis_exists(index: u32, axis: u32) -> Result<bool, ()> {
    unsafe {
        let axis_exists = vjoyinterface::GetVJDAxisExist(index, axis);
        if axis_exists == 0 {
                Ok(false)
        } else {
                Ok(true)
        }
    }
}

pub fn get_vjoystick_axis_min(index: u32, axis: u32) -> Result<i64, &'static str> {
    unsafe {
        let mut min: libc::c_long = 0;
        let min_raw_pointer = &mut min as *mut libc::c_long;
        let min_result = vjoyinterface::GetVJDAxisMin(index, axis, min_raw_pointer);
        if min_result == 0 {
                Err("Unable to get axis minimum")
        } else {
                Ok(min)
        }
    }
}

pub fn get_vjoystick_axis_max(index: u32, axis: u32) -> Result<i64, &'static str> {
    unsafe {
        let mut max: libc::c_long = 0;
        let max_raw_pointer = &mut max as *mut libc::c_long;
        let max_result = vjoyinterface::GetVJDAxisMax(index, axis, max_raw_pointer);
        if max_result == 0 {
                Err("Unable to get axis maximum: does the axis exist?")
        } else {
                Ok(max)
        }
    }
}

pub fn get_vjoystick_button_count(index: u32) -> Result<u8, ()> {
    unsafe {
        let num_buttons = vjoyinterface::GetVJDButtonNumber(index);

        Ok(num_buttons as u8)
    }
}

pub fn get_vjoystick_status(index: u32) -> vjoyinterface::Enum_VjdStat {
    unsafe {
        let joystick_status = vjoyinterface::GetVJDStatus(index);

        joystick_status
    }
}

pub fn reset_vjoystick(index: u32) -> Result<(), &'static str> {
    unsafe {
        let reset_result = vjoyinterface::ResetVJD(index);
        if reset_result == 0 {
            return Err("vJoy reset function returned failure");
        }
    }

    Ok(())
}

pub fn set_vjoystick_axis(index: u32, axis: u32, value: i64) -> Result<(), ()> {
    unsafe {
        let set_x_result = vjoyinterface::SetAxis(value, index, axis);
        if set_x_result == 0 {
            return Err(());
        }
    }

    Ok(())
}

pub fn set_vjoystick_button(index: u32, button: u8, value: i32) -> Result<(), ()> {
    unsafe {
        let set_result = vjoyinterface::SetBtn(value, index, button);
        if set_result == 0 {
            return Err(());
        }
    }

    Ok(())
}


// Convenience functions

pub fn claim_vjoystick(index: u32) -> Result<(), &'static str> {
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

    Err("Virtual joystick is owned by someone else, missing, or in unknown state")
}