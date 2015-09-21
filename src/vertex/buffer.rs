use std::error::Error;
use std::fmt;
use std::ops::{Range, Deref, DerefMut};

use buffer::{Buffer, BufferSlice, BufferAny, BufferType, BufferMode, BufferCreationError, Content};
use vertex::{Vertex, VerticesSource, IntoVerticesSource, PerInstance};
use vertex::format::VertexFormat;

use backend::Facade;
use version::{Api, Version};
use CapabilitiesSource;

/// Error that can happen when creating a vertex buffer.
#[derive(Copy, Clone, Debug)]
pub enum CreationError {
    /// The vertex format is not supported by the backend.
    ///
    /// Anything 64bits-related may not be supported.
    FormatNotSupported,

    /// Error while creating the vertex buffer.
    BufferCreationError(BufferCreationError),
}

impl From<BufferCreationError> for CreationError {
    #[inline]
    fn from(err: BufferCreationError) -> CreationError {
        CreationError::BufferCreationError(err)
    }
}

impl fmt::Display for CreationError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &CreationError::FormatNotSupported => "The vertex format is not supported by the \
                                                   backend".fmt(formatter),
            &CreationError::BufferCreationError(error) => error.fmt(formatter),
        }
    }
}

impl Error for CreationError {
    fn description(&self) -> &str {
        match self {
            &CreationError::FormatNotSupported => "The vertex format is not supported by the \
                                                   backend",
            &CreationError::BufferCreationError(..) => "Error while creating the vertex buffer",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match self {
            &CreationError::FormatNotSupported => None,
            &CreationError::BufferCreationError(ref error) => Some(error),
        }
    }
}

/// A list of vertices loaded in the graphics card's memory.
#[derive(Debug)]
pub struct VertexBuffer<T> where T: Copy {
    buffer: Buffer<[T]>,
    bindings: VertexFormat,
}

/// Represents a slice of a `VertexBuffer`.
pub struct VertexBufferSlice<'b, T: 'b> where T: Copy {
    buffer: BufferSlice<'b, [T]>,
    bindings: &'b VertexFormat,
}

impl<'b, T: 'b> VertexBufferSlice<'b, T> where T: Copy + Content {
    /// Creates a marker that instructs glium to use multiple instances.
    ///
    /// Instead of calling `surface.draw(&vertex_buffer.slice(...).unwrap(), ...)` 
    /// you can call `surface.draw(vertex_buffer.slice(...).unwrap().per_instance(), ...)`. 
    /// This will draw one instance of the geometry for each element in this buffer slice. 
    /// The attributes are still passed to the vertex shader, but each entry is passed 
    /// for each different instance.
    #[inline]
    pub fn per_instance(&'b self) -> Result<PerInstance, InstancingNotSupported> {
        // TODO: don't check this here
        if !(self.get_context().get_version() >= &Version(Api::Gl, 3, 3)) &&
            !self.get_context().get_extensions().gl_arb_instanced_arrays
        {
            return Err(InstancingNotSupported);
        }

        Ok(PerInstance(self.buffer.as_slice_any(), &self.bindings))
    }
}

impl<T> VertexBuffer<T> where T: Vertex {
    /// Builds a new vertex buffer.
    ///
    /// Note that operations such as `write` will be very slow. If you want to modify the buffer
    /// from time to time, you should use the `dynamic` function instead.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[macro_use]
    /// # extern crate glium;
    /// # extern crate glutin;
    /// # fn main() {
    /// #[derive(Copy, Clone)]
    /// struct Vertex {
    ///     position: [f32; 3],
    ///     texcoords: [f32; 2],
    /// }
    ///
    /// implement_vertex!(Vertex, position, texcoords);
    ///
    /// # let display: glium::Display = unsafe { ::std::mem::uninitialized() };
    /// let vertex_buffer = glium::VertexBuffer::new(&display, &[
    ///     Vertex { position: [0.0,  0.0, 0.0], texcoords: [0.0, 1.0] },
    ///     Vertex { position: [5.0, -3.0, 2.0], texcoords: [1.0, 0.0] },
    /// ]);
    /// # }
    /// ```
    ///
    #[inline]
    pub fn new<F>(facade: &F, data: &[T]) -> Result<VertexBuffer<T>, CreationError>
                  where F: Facade
    {
        VertexBuffer::new_impl(facade, data, BufferMode::Default)
    }

    /// Builds a new vertex buffer.
    ///
    /// This function will create a buffer that is intended to be modified frequently.
    #[inline]
    pub fn dynamic<F>(facade: &F, data: &[T]) -> Result<VertexBuffer<T>, CreationError>
                      where F: Facade
    {
        VertexBuffer::new_impl(facade, data, BufferMode::Dynamic)
    }

    /// Builds a new vertex buffer.
    #[inline]
    pub fn persistent<F>(facade: &F, data: &[T]) -> Result<VertexBuffer<T>, CreationError>
                         where F: Facade
    {
        VertexBuffer::new_impl(facade, data, BufferMode::Persistent)
    }

    /// Builds a new vertex buffer.
    #[inline]
    pub fn immutable<F>(facade: &F, data: &[T]) -> Result<VertexBuffer<T>, CreationError>
                        where F: Facade
    {
        VertexBuffer::new_impl(facade, data, BufferMode::Immutable)
    }

    #[inline]
    fn new_impl<F>(facade: &F, data: &[T], mode: BufferMode)
                   -> Result<VertexBuffer<T>, CreationError>
                   where F: Facade
    {
        if !T::is_supported(facade) {
            return Err(CreationError::FormatNotSupported);
        }

        let buffer = try!(Buffer::new(facade, data, BufferType::ArrayBuffer, mode));
        Ok(buffer.into())
    }

    /// Builds an empty vertex buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty<F>(facade: &F, elements: usize) -> Result<VertexBuffer<T>, CreationError>
                    where F: Facade
    {
        VertexBuffer::empty_impl(facade, elements, BufferMode::Default)
    }

    /// Builds an empty vertex buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_dynamic<F>(facade: &F, elements: usize) -> Result<VertexBuffer<T>, CreationError>
                            where F: Facade
    {
        VertexBuffer::empty_impl(facade, elements, BufferMode::Dynamic)
    }

    /// Builds an empty vertex buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_persistent<F>(facade: &F, elements: usize)
                               -> Result<VertexBuffer<T>, CreationError>
                               where F: Facade
    {
        VertexBuffer::empty_impl(facade, elements, BufferMode::Persistent)
    }

    /// Builds an empty vertex buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_immutable<F>(facade: &F, elements: usize) -> Result<VertexBuffer<T>, CreationError>
                              where F: Facade
    {
        VertexBuffer::empty_impl(facade, elements, BufferMode::Immutable)
    }

    #[inline]
    fn empty_impl<F>(facade: &F, elements: usize, mode: BufferMode)
                     -> Result<VertexBuffer<T>, CreationError>
                     where F: Facade
    {
        if !T::is_supported(facade) {
            return Err(CreationError::FormatNotSupported);
        }

        let buffer = try!(Buffer::empty_array(facade, BufferType::ArrayBuffer, elements, mode));
        Ok(buffer.into())
    }
}

impl<T> VertexBuffer<T> where T: Copy {
    /// Builds a new vertex buffer from an indeterminate data type and bindings.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate glium;
    /// # extern crate glutin;
    /// # fn main() {
    /// use std::borrow::Cow;
    ///
    /// let bindings = Cow::Owned(vec![(
    ///         Cow::Borrowed("position"), 0,
    ///         glium::vertex::AttributeType::F32F32,
    ///     ), (
    ///         Cow::Borrowed("color"), 2 * ::std::mem::size_of::<f32>(),
    ///         glium::vertex::AttributeType::F32,
    ///     ),
    /// ]);
    ///
    /// # let display: glium::Display = unsafe { ::std::mem::uninitialized() };
    /// let data = vec![
    ///     1.0, -0.3, 409.0,
    ///     -0.4, 2.8, 715.0f32
    /// ];
    ///
    /// let vertex_buffer = unsafe {
    ///     glium::VertexBuffer::new_raw(&display, &data, bindings, 3 * ::std::mem::size_of::<f32>())
    /// };
    /// # }
    /// ```
    ///
    #[inline]
    pub unsafe fn new_raw<F>(facade: &F, data: &[T],
                             bindings: VertexFormat, elements_size: usize)
                             -> Result<VertexBuffer<T>, CreationError>
                             where F: Facade
    {
        // FIXME: check that the format is supported

        Ok(VertexBuffer {
            buffer: try!(Buffer::new(facade, data, BufferType::ArrayBuffer,
                                         BufferMode::Default)),
            bindings: bindings,
        })
    }

    /// Dynamic version of `new_raw`.
    #[inline]
    pub unsafe fn new_raw_dynamic<F>(facade: &F, data: &[T],
                                     bindings: VertexFormat, elements_size: usize)
                                     -> Result<VertexBuffer<T>, CreationError>
                                     where F: Facade
    {
        // FIXME: check that the format is supported

        Ok(VertexBuffer {
            buffer: try!(Buffer::new(facade, data, BufferType::ArrayBuffer,
                                         BufferMode::Dynamic)),
            bindings: bindings,
        })
    }

    /// Accesses a slice of the buffer.
    ///
    /// Returns `None` if the slice is out of range.
    #[inline]
    pub fn slice(&self, range: Range<usize>) -> Option<VertexBufferSlice<T>> {
        let slice = match self.buffer.slice(range) {
            None => return None,
            Some(s) => s
        };

        Some(VertexBufferSlice {
            buffer: slice,
            bindings: &self.bindings,
        })
    }

    /// Returns the associated `VertexFormat`.
    #[inline]
    pub fn get_bindings(&self) -> &VertexFormat {
        &self.bindings
    }

    /// Creates a marker that instructs glium to use multiple instances.
    ///
    /// Instead of calling `surface.draw(&vertex_buffer, ...)` you can call
    /// `surface.draw(vertex_buffer.per_instance(), ...)`. This will draw one instance of the
    /// geometry for each element in this buffer. The attributes are still passed to the
    /// vertex shader, but each entry is passed for each different instance.
    #[inline]
    pub fn per_instance(&self) -> Result<PerInstance, InstancingNotSupported> {
        // TODO: don't check this here
        if !(self.buffer.get_context().get_version() >= &Version(Api::Gl, 3, 3)) &&
            !self.buffer.get_context().get_extensions().gl_arb_instanced_arrays
        {
            return Err(InstancingNotSupported);
        }

        Ok(PerInstance(self.buffer.as_slice_any(), &self.bindings))
    }
}

impl<T> VertexBuffer<T> where T: Copy + Send + 'static {
    /// DEPRECATED: use `.into()` instead.
    /// Discard the type information and turn the vertex buffer into a `VertexBufferAny`.
    #[inline]
    pub fn into_vertex_buffer_any(self) -> VertexBufferAny {
        VertexBufferAny {
            buffer: self.buffer.into(),
            bindings: self.bindings,
        }
    }
}

impl<T> From<Buffer<[T]>> for VertexBuffer<T> where T: Vertex + Copy {
    #[inline]
    fn from(buffer: Buffer<[T]>) -> VertexBuffer<T> {
        assert!(T::is_supported(buffer.get_context()));

        let bindings = <T as Vertex>::build_bindings();

        VertexBuffer {
            buffer: buffer,
            bindings: bindings,
        }
    }
}

impl<T> Deref for VertexBuffer<T> where T: Copy {
    type Target = Buffer<[T]>;

    #[inline]
    fn deref(&self) -> &Buffer<[T]> {
        &self.buffer
    }
}

impl<T> DerefMut for VertexBuffer<T> where T: Copy {
    #[inline]
    fn deref_mut(&mut self) -> &mut Buffer<[T]> {
        &mut self.buffer
    }
}

impl<'a, T> IntoVerticesSource<'a> for &'a VertexBuffer<T> where T: Copy {
    #[inline]
    fn into_vertices_source(self) -> VerticesSource<'a> {
        VerticesSource::VertexBuffer(self.buffer.as_slice_any(), &self.bindings, false)
    }
}

impl<'a, T> Deref for VertexBufferSlice<'a, T> where T: Copy {
    type Target = BufferSlice<'a, [T]>;

    #[inline]
    fn deref(&self) -> &BufferSlice<'a, [T]> {
        &self.buffer
    }
}

impl<'a, T> DerefMut for VertexBufferSlice<'a, T> where T: Copy {
    #[inline]
    fn deref_mut(&mut self) -> &mut BufferSlice<'a, [T]> {
        &mut self.buffer
    }
}

impl<'a, T> IntoVerticesSource<'a> for VertexBufferSlice<'a, T> where T: Copy {
    #[inline]
    fn into_vertices_source(self) -> VerticesSource<'a> {
        VerticesSource::VertexBuffer(self.buffer.as_slice_any(), &self.bindings, false)
    }
}

/// A list of vertices loaded in the graphics card's memory.
///
/// Contrary to `VertexBuffer`, this struct doesn't know about the type of data
/// inside the buffer. Therefore you can't map or read it.
///
/// This struct is provided for convenience, so that you can have a `Vec<VertexBufferAny>`,
/// or return a `VertexBufferAny` instead of a `VertexBuffer<MyPrivateVertexType>`.
#[derive(Debug)]
pub struct VertexBufferAny {
    buffer: BufferAny,
    bindings: VertexFormat,
}

impl VertexBufferAny {
    /// Returns the number of bytes between two consecutive elements in the buffer.
    #[inline]
    pub fn get_elements_size(&self) -> usize {
        self.buffer.get_elements_size()
    }

    /// Returns the number of elements in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.get_elements_count()
    }

    /// Returns the associated `VertexFormat`.
    #[inline]
    pub fn get_bindings(&self) -> &VertexFormat {
        &self.bindings
    }

    /// Turns the vertex buffer into a `VertexBuffer` without checking the type.
    #[inline]
    pub unsafe fn into_vertex_buffer<T: Copy>(self) -> VertexBuffer<T> {
        unimplemented!();
    }

    /// Creates a marker that instructs glium to use multiple instances.
    ///
    /// Instead of calling `surface.draw(&vertex_buffer, ...)` you can call
    /// `surface.draw(vertex_buffer.per_instance(), ...)`. This will draw one instance of the
    /// geometry for each element in this buffer. The attributes are still passed to the
    /// vertex shader, but each entry is passed for each different instance.
    #[inline]
    pub fn per_instance(&self) -> Result<PerInstance, InstancingNotSupported> {
        // TODO: don't check this here
        if !(self.buffer.get_context().get_version() >= &Version(Api::Gl, 3, 3)) &&
            !self.buffer.get_context().get_extensions().gl_arb_instanced_arrays
        {
            return Err(InstancingNotSupported);
        }

        Ok(PerInstance(self.buffer.as_slice_any(), &self.bindings))
    }
}

impl<T> From<VertexBuffer<T>> for VertexBufferAny where T: Copy + Send + 'static {
    #[inline]
    fn from(buf: VertexBuffer<T>) -> VertexBufferAny {
        buf.into_vertex_buffer_any()
    }
}

impl<T> From<Buffer<[T]>> for VertexBufferAny where T: Vertex + Copy + Send + 'static {
    #[inline]
    fn from(buf: Buffer<[T]>) -> VertexBufferAny {
        let buf: VertexBuffer<T> = buf.into();
        buf.into_vertex_buffer_any()
    }
}

impl<'a> IntoVerticesSource<'a> for &'a VertexBufferAny {
    #[inline]
    fn into_vertices_source(self) -> VerticesSource<'a> {
        VerticesSource::VertexBuffer(self.buffer.as_slice_any(), &self.bindings, false)
    }
}

/// Instancing is not supported by the backend.
#[derive(Debug, Copy, Clone)]
pub struct InstancingNotSupported;
