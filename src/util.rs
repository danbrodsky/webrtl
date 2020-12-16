use crate::config::STATE;

#[macro_use]
macro_rules! get {
    ( $mv: expr ) => {
        STATE.lock().unwrap().get($mv).unwrap()
    };
}

pub fn get_n_to_m(var: &str, n: usize, m: usize) -> Vec<u8> {
    let s = STATE.lock().unwrap();
    let mut out = vec!();
    for b in n..m {
        out.push(s.get(&format!("{}[{}]", var, b)).unwrap().val)
    }
    return out
}

// TODO: check that var being set is Model Input
pub fn set(var: &str, val: u8) {
    STATE.lock().unwrap().get_mut(var.into()).unwrap().val = val;
}

pub fn set_n(var: &str, n: usize, val: u8) {
    STATE.lock().unwrap().get_mut(&format!("{}[{}]", var, n)).unwrap().val = val;
}

pub fn set_n_to_m(var: &str, n: usize, m: usize, val: Vec<u8>) {

    // trace!("setting {} to val {:#?}", var, val);
    for b in n..m {
        STATE.lock().unwrap().get_mut(&format!("{}[{}]", var, b)).unwrap().val = val[b-n];
    }
}

// TODO: make this generic?
pub fn to_bit_vec(v: u64) -> Vec<u8> {
    let mut bv: Vec<u8> = vec!();
    let mut n = v;
    for _ in 0..64 {
        bv.push((n & 0x1) as u8);
        n = n >> 0x1;
    }
    return bv
}
