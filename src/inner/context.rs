use std::collections::HashMap;
use std::cell::{RefCell, Ref, RefMut};

use super::RbRef as RbRefData;


pub struct RefId(u64);

pub struct RbContext {
    objects: HashMap<RefId, RefCell<RbRefData>>,
    next_id: RefId,
}

impl RbContext {
    // pub fn new_object(&mut self)


}

pub struct RbRef<'a> {
    context_: &'a RbContext,
    id_: RefId,
    data_: Ref<'a, RbRefData>,
}

impl<'a> RbRef<'a> {

}

pub struct RbRefMut<'a> {
    context_: &'a RbContext,
    id_: RefId,
    data_: RefMut<'a, RbRefData>,
}
impl<'a> RbRefMut<'a> {
    // pub fn as_array(&self) -> Option<&'a RbArray> {  }
}

// macro_rules! generate_rb_ref_impl {
//     () => {
        
//     };
// }

