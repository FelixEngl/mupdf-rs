use std::mem;
use std::os::raw::c_void;

use mupdf_sys::*;

use crate::{context, Error, Matrix, Point, Rect, StrokeState};

pub trait PathWalker {
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, ex: f32, ey: f32);
    fn close(&mut self);
}

extern "C" fn path_walk_move_to(_ctx: *mut fz_context, arg: *mut c_void, x: f32, y: f32) {
    let walker: Box<&mut dyn PathWalker> = unsafe { mem::transmute(arg) };
    walker.move_to(x, y);
    mem::forget(walker);
}

extern "C" fn path_walk_line_to(_ctx: *mut fz_context, arg: *mut c_void, x: f32, y: f32) {
    let walker: Box<&mut dyn PathWalker> = unsafe { mem::transmute(arg) };
    walker.line_to(x, y);
    mem::forget(walker);
}

extern "C" fn path_walk_curve_to(
    _ctx: *mut fz_context,
    arg: *mut c_void,
    cx1: f32,
    cy1: f32,
    cx2: f32,
    cy2: f32,
    ex: f32,
    ey: f32,
) {
    let walker: Box<&mut dyn PathWalker> = unsafe { mem::transmute(arg) };
    walker.curve_to(cx1, cy1, cx2, cy2, ex, ey);
    mem::forget(walker);
}

extern "C" fn path_walk_close(_ctx: *mut fz_context, arg: *mut c_void) {
    let walker: Box<&mut dyn PathWalker> = unsafe { mem::transmute(arg) };
    walker.close();
    mem::forget(walker);
}

#[derive(Debug)]
pub struct Path {
    pub(crate) inner: *mut fz_path,
}

impl Path {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_path) -> Self {
        Self { inner: ptr }
    }

    pub fn new() -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_path(context())) };
        Ok(Self { inner })
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_clone_path(context(), self.inner)) };
        Ok(Self { inner })
    }

    pub fn walk(&self, walker: &mut dyn PathWalker) -> Result<(), Error> {
        unsafe {
            let c_walker = fz_path_walker {
                moveto: Some(path_walk_move_to),
                lineto: Some(path_walk_line_to),
                curveto: Some(path_walk_curve_to),
                closepath: Some(path_walk_close),
                quadto: None,
                curvetov: None,
                curvetoy: None,
                rectto: None,
            };
            let raw_ptr = Box::into_raw(Box::new(walker));
            ffi_try!(mupdf_walk_path(
                context(),
                self.inner,
                &c_walker,
                raw_ptr as _
            ));
            let _ = Box::from_raw(raw_ptr);
        }
        Ok(())
    }

    pub fn current_point(&self) -> Point {
        let inner = unsafe { fz_currentpoint(context(), self.inner) };
        inner.into()
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_moveto(context(), self.inner, x, y));
        }
        Ok(())
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_lineto(context(), self.inner, x, y));
        }
        Ok(())
    }

    pub fn curve_to(
        &mut self,
        cx1: f32,
        cy1: f32,
        cx2: f32,
        cy2: f32,
        ex: f32,
        ey: f32,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_curveto(
                context(),
                self.inner,
                cx1,
                cy1,
                cx2,
                cy2,
                ex,
                ey
            ));
        }
        Ok(())
    }

    pub fn curve_to_v(&mut self, cx: f32, cy: f32, ex: f32, ey: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_curvetov(context(), self.inner, cx, cy, ex, ey));
        }
        Ok(())
    }

    pub fn curve_to_y(&mut self, cx: f32, cy: f32, ex: f32, ey: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_curvetoy(context(), self.inner, cx, cy, ex, ey));
        }
        Ok(())
    }

    pub fn rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_rectto(context(), self.inner, x1, y1, x2, y2));
        }
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_closepath(context(), self.inner));
        }
        Ok(())
    }

    pub fn transform(&mut self, mat: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_transform_path(context(), self.inner, mat.into()));
        }
        Ok(())
    }

    pub fn bounds(&self, stroke: &StrokeState, ctm: &Matrix) -> Result<Rect, Error> {
        let rect = unsafe {
            ffi_try!(mupdf_bound_path(
                context(),
                self.inner,
                stroke.inner,
                ctm.into()
            ))
        };
        Ok(rect.into())
    }

    pub fn trim(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_trim_path(context(), self.inner));
        }
        Ok(())
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_path(context(), self.inner);
            }
        }
    }
}

impl Clone for Path {
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::{Path, PathWalker};

    struct TestPathWalker {
        move_to: bool,
        line_to: bool,
        curve_to: bool,
        close: bool,
    }

    impl PathWalker for TestPathWalker {
        fn move_to(&mut self, x: f32, y: f32) {
            if x == 0.0 && y == 0.0 {
                self.move_to = true;
            }
        }

        fn line_to(&mut self, x: f32, y: f32) {
            if x == 10.0 && y == 10.0 {
                self.line_to = true;
            }
        }

        #[allow(unused_variables)]
        fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, ex: f32, ey: f32) {}

        fn close(&mut self) {
            self.close = true;
        }
    }

    #[test]
    fn test_walk_path() {
        let mut path = Path::new().unwrap();
        path.move_to(0.0, 0.0).unwrap();
        path.line_to(10.0, 10.0).unwrap();
        path.close().unwrap();
        let mut walker = TestPathWalker {
            move_to: false,
            line_to: false,
            curve_to: false,
            close: false,
        };
        path.walk(&mut walker).unwrap();
        assert!(walker.move_to);
        assert!(walker.line_to);
        assert!(walker.close);
        assert!(!walker.curve_to);
    }
}
