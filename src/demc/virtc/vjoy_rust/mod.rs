mod vjoyinterface;

extern crate libc;

// Rustifying vJoy wrapper functions + convenience functions

// Wrapper functions

pub fn get_vjoy_version() -> i16 {
    unsafe {
        vjoyinterface::GetvJoyVersion()
    }
}

pub fn is_vjoy_enabled() -> bool {
    unsafe {
        match vjoyinterface::vJoyEnabled() {
            0 => false,
            _ => true
        }
    }
}

/*@todo
    pub fn GetvJoyProductString() -> *mut libc::c_void;
    pub fn GetvJoyManufacturerString() -> *mut libc::c_void;
    pub fn GetvJoySerialNumberString() -> *mut libc::c_void;
    pub fn DriverMatch(DllVer: *mut libc::c_ushort,
                       DrvVer: *mut libc::c_ushort) -> libc::c_int;
    pub fn RegisterRemovalCB(cb: RemovalCB, data: *mut libc::c_void) -> ();
    pub fn GetVJDDiscPovNumber(rID: libc::c_uint) -> libc::c_int;
    pub fn GetVJDContPovNumber(rID: libc::c_uint) -> libc::c_int;
*/

pub fn get_vjoystick_axis_exists(index: u32, axis: u32) -> bool {
    unsafe {
        match vjoyinterface::GetVJDAxisExist(index, axis){
            0 => false,
            _ => true
        }
    }
}

pub fn get_vjoystick_axis_max(index: u32, axis: u32) -> Result<i64, ()> {
    unsafe {
        let mut max: libc::c_long = 0;
        let max_raw_pointer = &mut max as *mut libc::c_long;
        match vjoyinterface::GetVJDAxisMax(index, axis, max_raw_pointer) {
            0 => Err(()),
            _ => Ok(max as i64)
        }
    }
}

pub fn get_vjoystick_axis_min(index: u32, axis: u32) -> Result<i64, ()> {
    unsafe {
        let mut min: libc::c_long = 0;
        let min_raw_pointer = &mut min as *mut libc::c_long;
        match vjoyinterface::GetVJDAxisMin(index, axis, min_raw_pointer) {
            0 => Err(()),
            _ => Ok(min as i64)
        }
    }
}

pub fn acquire_vjoystick(index: u32) -> Result<(), ()> {
    unsafe {
        match vjoyinterface::AcquireVJD(index) {
            0 => Err(()),
            _ => Ok(())
        }
    }
}

pub fn relinquish_vjoystick(index: u32) {
    unsafe {
        vjoyinterface::RelinquishVJD(index)
    }
}

pub fn get_vjoystick_button_count(index: u32) -> u8 {
    unsafe {
        vjoyinterface::GetVJDButtonNumber(index) as u8
    }
}

//@todo     pub fn UpdateVJD(rID: libc::c_uint, pData: *mut libc::c_void) -> libc::c_int;

pub enum VjoystickStatus {
    Owned,      // Owned by this application
    Free,       // Owned by no one
    Busy,       // Owned by someone else; can't be acquired by us
    Missing,    // Doesn't exist, or driver is down
    Unknown     // Unknown
}
pub fn get_vjoystick_status(index: u32) -> VjoystickStatus {
    unsafe {
        match vjoyinterface::GetVJDStatus(index) {
            vjoyinterface::VJD_STAT_OWN => VjoystickStatus::Owned,
            vjoyinterface::VJD_STAT_FREE => VjoystickStatus::Free,
            vjoyinterface::VJD_STAT_BUSY => VjoystickStatus::Busy,
            vjoyinterface::VJD_STAT_MISS => VjoystickStatus::Missing,
            vjoyinterface::VJD_STAT_UNKN => VjoystickStatus::Unknown,
            _ => VjoystickStatus::Unknown
        }
    }
}

pub fn reset_vjoystick(index: u32) -> Result<(), ()> {
    unsafe {
        match vjoyinterface::ResetVJD(index) {
            0 => Err(()),
            _ => Ok(())
        }
    }
}

pub fn reset_all_vjoysticks() {
    unsafe {
        vjoyinterface::ResetAll()
    }
}

/*@todo
    pub fn ResetButtons(rID: libc::c_uint) -> libc::c_int;
    pub fn ResetPovs(rID: libc::c_uint) -> libc::c_int;
*/

pub fn set_vjoystick_axis(index: u32, axis: u32, value: i64) -> Result<(), ()> {
    unsafe {
        match vjoyinterface::SetAxis(value as libc::c_long, index, axis) {
            0 => Err(()),
            _ => Ok(())
        }
    }
}

pub fn set_vjoystick_button(index: u32, button: u8, value: i32) -> Result<(), ()> {
    unsafe {
        match vjoyinterface::SetBtn(value, index, button) {
            0 => Err(()),
            _ => Ok(())
        }
    }
}

/*@todo
    pub fn SetDiscPov(Value: libc::c_int, rID: libc::c_uint, nPov: libc::c_uchar) -> libc::c_int;
    pub fn SetContPov(Value: libc::c_ulong, rID: libc::c_uint, nPov: libc::c_uchar) -> libc::c_int;
*/


// Convenience functions

pub fn claim_vjoystick(index: u32) -> Result<(), &'static str> {
    match get_vjoystick_status(index) {
        VjoystickStatus::Free => {
            // Try to claim it
            match acquire_vjoystick(index) {
                Ok(_) => Ok(()),
                Err(_) => Err("Virtual joystick is available, but unable to acquire it")
            }
        },
        VjoystickStatus::Owned => {
            // We've already claimed it
            Ok(())
        },
        _ => Err("Virtual joystick is owned by someone else, missing, or in unknown state")
    }
}