// Copyright (C) 2016-2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use std::ptr;
use std::mem;
use std::ffi::{CStr, CString};
use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut, ToOwned};
use std::marker::PhantomData;

use value::*;

use glib;
use gst;

pub struct OwnedStructure(*mut Structure, PhantomData<Structure>);

impl OwnedStructure {
    pub fn new_empty(name: &str) -> OwnedStructure {
        let name_cstr = CString::new(name).unwrap();
        OwnedStructure(
            unsafe { gst::gst_structure_new_empty(name_cstr.as_ptr()) as *mut Structure },
            PhantomData,
        )
    }

    pub fn new(name: &str, values: &[(&str, Value)]) -> OwnedStructure {
        let mut structure = OwnedStructure::new_empty(name);

        for &(f, ref v) in values {
            structure.set(f, v.clone());
        }

        structure
    }

    pub fn from_string(s: &str) -> Option<OwnedStructure> {
        unsafe {
            let cstr = CString::new(s).unwrap();
            let structure = gst::gst_structure_from_string(cstr.as_ptr(), ptr::null_mut());
            if structure.is_null() {
                None
            } else {
                Some(OwnedStructure(structure as *mut Structure, PhantomData))
            }
        }
    }

    pub unsafe fn into_ptr(self) -> *mut gst::GstStructure {
        let ptr = self.0 as *mut Structure as *mut gst::GstStructure;
        mem::forget(self);

        ptr
    }
}

impl Deref for OwnedStructure {
    type Target = Structure;

    fn deref(&self) -> &Structure {
        unsafe { &*self.0 }
    }
}

impl DerefMut for OwnedStructure {
    fn deref_mut(&mut self) -> &mut Structure {
        unsafe { &mut *self.0 }
    }
}

impl AsRef<Structure> for OwnedStructure {
    fn as_ref(&self) -> &Structure {
        self.deref()
    }
}

impl AsMut<Structure> for OwnedStructure {
    fn as_mut(&mut self) -> &mut Structure {
        self.deref_mut()
    }
}

impl Clone for OwnedStructure {
    fn clone(&self) -> Self {
        OwnedStructure(
            unsafe { gst::gst_structure_copy(&(*self.0).0) as *mut Structure },
            PhantomData,
        )
    }
}

impl Drop for OwnedStructure {
    fn drop(&mut self) {
        unsafe { gst::gst_structure_free(&mut (*self.0).0) }
    }
}

impl fmt::Debug for OwnedStructure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl PartialEq for OwnedStructure {
    fn eq(&self, other: &OwnedStructure) -> bool {
        self.as_ref().eq(other)
    }
}

impl PartialEq<Structure> for OwnedStructure {
    fn eq(&self, other: &Structure) -> bool {
        self.as_ref().eq(other)
    }
}

impl Eq for OwnedStructure {}

impl Borrow<Structure> for OwnedStructure {
    fn borrow(&self) -> &Structure {
        unsafe { &*self.0 }
    }
}

impl BorrowMut<Structure> for OwnedStructure {
    fn borrow_mut(&mut self) -> &mut Structure {
        unsafe { &mut *self.0 }
    }
}

impl ToOwned for Structure {
    type Owned = OwnedStructure;

    fn to_owned(&self) -> OwnedStructure {
        OwnedStructure(
            unsafe { gst::gst_structure_copy(&self.0) as *mut Structure },
            PhantomData,
        )
    }
}

#[repr(C)]
pub struct Structure(gst::GstStructure);

impl Structure {
    pub unsafe fn from_borrowed_ptr<'a>(ptr: *const gst::GstStructure) -> &'a Structure {
        assert!(!ptr.is_null());

        &*(ptr as *mut Structure)
    }

    pub unsafe fn from_borrowed_mut_ptr<'a>(ptr: *mut gst::GstStructure) -> &'a mut Structure {
        assert!(!ptr.is_null());

        &mut *(ptr as *mut Structure)
    }

    pub fn to_string(&self) -> String {
        unsafe {
            let ptr = gst::gst_structure_to_string(&self.0);
            let s = CStr::from_ptr(ptr).to_str().unwrap().into();
            glib::g_free(ptr as glib::gpointer);

            s
        }
    }

    pub fn get<'a, T: ValueType<'a>>(&'a self, name: &str) -> Option<TypedValueRef<'a, T>> {
        self.get_value(name).and_then(TypedValueRef::from_value_ref)
    }

    pub fn get_value<'a>(&'a self, name: &str) -> Option<ValueRef<'a>> {
        unsafe {
            let name_cstr = CString::new(name).unwrap();

            let value = gst::gst_structure_get_value(&self.0, name_cstr.as_ptr());

            if value.is_null() {
                return None;
            }

            ValueRef::from_ptr(value)
        }
    }

    pub fn set<T: Into<Value>>(&mut self, name: &str, value: T) {
        unsafe {
            let name_cstr = CString::new(name).unwrap();
            let mut gvalue = value.into().into_raw();

            gst::gst_structure_take_value(&mut self.0, name_cstr.as_ptr(), &mut gvalue);
            mem::forget(gvalue);
        }
    }

    pub fn get_name(&self) -> &str {
        unsafe {
            let cstr = CStr::from_ptr(gst::gst_structure_get_name(&self.0));
            cstr.to_str().unwrap()
        }
    }

    pub fn has_field(&self, field: &str) -> bool {
        unsafe {
            let cstr = CString::new(field).unwrap();
            gst::gst_structure_has_field(&self.0, cstr.as_ptr()) == glib::GTRUE
        }
    }

    pub fn remove_field(&mut self, field: &str) {
        unsafe {
            let cstr = CString::new(field).unwrap();
            gst::gst_structure_remove_field(&mut self.0, cstr.as_ptr());
        }
    }

    pub fn remove_all_fields(&mut self) {
        unsafe {
            gst::gst_structure_remove_all_fields(&mut self.0);
        }
    }

    pub fn fields(&self) -> FieldIterator {
        FieldIterator::new(self)
    }

    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    fn get_nth_field_name(&self, idx: u32) -> Option<&str> {
        unsafe {
            let field_name = gst::gst_structure_nth_field_name(&self.0, idx);
            if field_name.is_null() {
                return None;
            }

            let cstr = CStr::from_ptr(field_name);
            Some(cstr.to_str().unwrap())
        }
    }

    fn n_fields(&self) -> u32 {
        unsafe { gst::gst_structure_n_fields(&self.0) as u32 }
    }

    // TODO: Various operations
}

impl fmt::Debug for Structure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl PartialEq for Structure {
    fn eq(&self, other: &Structure) -> bool {
        (unsafe { gst::gst_structure_is_equal(&self.0, &other.0) } == glib::GTRUE)
    }
}

impl Eq for Structure {}

pub struct FieldIterator<'a> {
    structure: &'a Structure,
    idx: u32,
    n_fields: u32,
}

impl<'a> FieldIterator<'a> {
    pub fn new(structure: &'a Structure) -> FieldIterator<'a> {
        let n_fields = structure.n_fields();

        FieldIterator {
            structure: structure,
            idx: 0,
            n_fields: n_fields,
        }
    }
}

impl<'a> Iterator for FieldIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.idx >= self.n_fields {
            return None;
        }

        if let Some(field_name) = self.structure.get_nth_field_name(self.idx) {
            self.idx += 1;
            Some(field_name)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.idx == self.n_fields {
            return (0, Some(0));
        }

        let remaining = (self.n_fields - self.idx) as usize;

        (remaining, Some(remaining))
    }
}

impl<'a> DoubleEndedIterator for FieldIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.idx == self.n_fields {
            return None;
        }

        self.n_fields -= 1;
        if let Some(field_name) = self.structure.get_nth_field_name(self.n_fields) {
            Some(field_name)
        } else {
            None
        }
    }
}

impl<'a> ExactSizeIterator for FieldIterator<'a> {}

pub struct Iter<'a> {
    iter: FieldIterator<'a>,
}

impl<'a> Iter<'a> {
    pub fn new(structure: &'a Structure) -> Iter<'a> {
        Iter {
            iter: FieldIterator::new(structure),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, ValueRef<'a>);

    fn next(&mut self) -> Option<(&'a str, ValueRef<'a>)> {
        if let Some(f) = self.iter.next() {
            let v = self.iter.structure.get_value(f);
            Some((f, v.unwrap()))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(f) = self.iter.next_back() {
            let v = self.iter.structure.get_value(f);
            Some((f, v.unwrap()))
        } else {
            None
        }
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn new_set_get() {
        unsafe { gst::gst_init(ptr::null_mut(), ptr::null_mut()) };

        let mut s = OwnedStructure::new_empty("test");
        assert_eq!(s.get_name(), "test");

        s.set("f1", "abc");
        s.set("f2", String::from("bcd"));
        s.set("f3", 123i32);

        assert_eq!(s.get::<&str>("f1").unwrap().get(), "abc");
        assert_eq!(s.get::<&str>("f2").unwrap().get(), "bcd");
        assert_eq!(s.get::<i32>("f3").unwrap().get(), 123i32);
        assert_eq!(s.fields().collect::<Vec<_>>(), vec!["f1", "f2", "f3"]);
        assert_eq!(
            s.iter()
                .map(|(f, v)| (f, Value::from_value_ref(&v)))
                .collect::<Vec<_>>(),
            vec![
                ("f1", Value::new("abc")),
                ("f2", Value::new("bcd")),
                ("f3", Value::new(123i32)),
            ]
        );

        let s2 = OwnedStructure::new(
            "test",
            &[
                ("f1", "abc".into()),
                ("f2", "bcd".into()),
                ("f3", 123i32.into()),
            ],
        );
        assert_eq!(s, s2);
    }
}
