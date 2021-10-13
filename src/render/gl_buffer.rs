use gles3::gles::*;
use gl::types::*;
use std::os::raw::c_void;

/// mapped gl uniform buffer
#[derive(Copy, Clone, Debug)]
pub struct GlBuffer {
    pub target: u32,
    pub buffer: u32,
    pub size: isize
}

impl GlBuffer {
    pub fn create(data_size: isize, data: *const u8) -> GlBuffer {
        let mut buffer: u32 = 0;
        unsafe {
            glGenBuffers(1, &mut buffer);
            glBindBuffer(GL_UNIFORM_BUFFER, buffer);
            glBufferData(GL_UNIFORM_BUFFER, data_size as GLsizeiptr, data as *const _ as *const c_void, GL_STATIC_DRAW);
            glBindBuffer(GL_UNIFORM_BUFFER, 0);
        }
        GlBuffer {
            target: GL_UNIFORM_BUFFER,
            buffer,
            size: data_size
        }
    }

    pub fn update(&self, update_data_size: isize, data: *const u8) {
        if update_data_size > self.size {
            panic!("gl_buffer update failed. update_data_size > size");
        }

        unsafe {
            glBindBuffer(self.target, self.buffer);
            glBufferSubData(self.target, 0,  update_data_size as GLsizeiptr, data as *const _ as *const c_void);
            glBindBuffer(self.target, 0);
        }
    }

    pub fn map_buffer(&self) -> *mut u8 {
        let mut data: *const c_void = std::ptr::null();
        unsafe {
            glBindBuffer(self.target, self.buffer);
            data = glMapBufferRange(self.target, 0, self.size as GLsizeiptr, GL_MAP_WRITE_BIT | GL_MAP_INVALIDATE_BUFFER_BIT);
            glBindBuffer(self.target, 0);
        }
        if data.is_null() {
            error!("failed to map buffer");
        }
        return data as *const _ as *mut u8;
    }

    pub fn unmap_buffer(&self) {
        unsafe {
            glBindBuffer(self.target, self.buffer);
            if glUnmapBuffer(self.target) == GL_FALSE {
                error!("failed to unmap buffer");
            }
            glBindBuffer(self.target, 0);
        }
    }

    fn destroy(&mut self) {
        if self.buffer == 0 {
            return;
        }
        unsafe {
            glDeleteBuffers(1, &mut self.buffer);
        }
        self.buffer = 0;
    }
}