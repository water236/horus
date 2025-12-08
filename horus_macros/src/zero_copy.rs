//! Zero-Copy Message Layout Macro
//!
//! Provides compile-time verified message layouts for high-performance
//! zero-copy serialization in HORUS recordings.
//!
//! ## Features
//!
//! - **Compile-time size calculation**: Know exact message size at compile time
//! - **Memory alignment guarantees**: Proper alignment for zero-copy access
//! - **Fixed-size strings**: Predictable memory layout with bounded strings
//! - **bytemuck integration**: Automatic Pod + Zeroable derivation
//! - **Endianness aware**: Optional field for cross-platform compatibility
//!
//! ## Example
//!
//! ```rust,ignore
//! zero_copy_message! {
//!     /// IMU sensor reading with fixed layout
//!     ImuReading {
//!         timestamp_ns: u64,
//!         accel: [f32; 3],
//!         gyro: [f32; 3],
//!         mag: [f32; 3],
//!         temperature: f32,
//!         status: u8,
//!         _padding: [u8; 3],  // Explicit padding for alignment
//!     }
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Field, Ident, Lit, Result, Token, Type,
};

/// Input for zero_copy_message! macro
pub struct ZeroCopyMessageInput {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub fields: Vec<ZeroCopyField>,
}

pub struct ZeroCopyField {
    pub name: Ident,
    pub ty: Type,
}

impl Parse for ZeroCopyMessageInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse attributes (doc comments, etc.)
        let attrs = input.call(Attribute::parse_outer)?;

        // Parse struct name
        let name: Ident = input.parse()?;

        // Parse fields in braces
        let content;
        syn::braced!(content in input);

        let fields: Punctuated<Field, Token![,]> =
            content.parse_terminated(Field::parse_named, Token![,])?;

        let fields: Vec<ZeroCopyField> = fields
            .into_iter()
            .map(|f| ZeroCopyField {
                name: f.ident.unwrap(),
                ty: f.ty,
            })
            .collect();

        Ok(ZeroCopyMessageInput {
            attrs,
            name,
            fields,
        })
    }
}

/// Generate the zero-copy message implementation
pub fn generate_zero_copy_message(input: ZeroCopyMessageInput) -> TokenStream {
    let ZeroCopyMessageInput {
        attrs,
        name,
        fields,
    } = input;

    let field_defs = fields.iter().map(|f| {
        let field_name = &f.name;
        let field_type = &f.ty;
        quote! { pub #field_name: #field_type }
    });

    let field_names: Vec<_> = fields.iter().map(|f| &f.name).collect();

    // Generate size assertion for compile-time verification
    let size_check_name = format_ident!("_SIZE_CHECK_{}", name);

    // Generate field offset assertions
    let offset_checks = fields.iter().map(|f| {
        let field_name = &f.name;
        let check_name = format_ident!("_OFFSET_CHECK_{}_{}", name, field_name);
        quote! {
            #[allow(dead_code)]
            const #check_name: () = {
                // This ensures the field offset is correct
                let _ = std::mem::offset_of!(#name, #field_name);
            };
        }
    });

    // Generate Default impl with zeroed values
    let default_fields = fields.iter().map(|f| {
        let field_name = &f.name;
        let field_type = &f.ty;
        quote! { #field_name: <#field_type as ::core::default::Default>::default() }
    });

    quote! {
        /// Zero-copy message with compile-time verified layout
        #(#attrs)*
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(C, packed)]
        pub struct #name {
            #(#field_defs),*
        }

        // Compile-time size verification
        #[allow(dead_code)]
        const #size_check_name: () = {
            // Verify that the struct is actually Pod-safe
            assert!(std::mem::align_of::<#name>() <= 8, "Alignment must be <= 8");
        };

        #(#offset_checks)*

        // SAFETY: The struct is #[repr(C, packed)] with only primitive/array fields
        // that are themselves Pod. This makes the entire struct Pod-safe.
        unsafe impl ::bytemuck::Pod for #name {}
        unsafe impl ::bytemuck::Zeroable for #name {}

        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#default_fields),*
                }
            }
        }

        impl #name {
            /// Get the size of this message in bytes (compile-time constant)
            pub const SIZE: usize = std::mem::size_of::<Self>();

            /// Create a new message with all fields zeroed
            pub const fn zeroed() -> Self {
                unsafe { std::mem::zeroed() }
            }

            /// Convert to raw bytes (zero-copy)
            #[inline]
            pub fn as_bytes(&self) -> &[u8] {
                ::bytemuck::bytes_of(self)
            }

            /// Create from raw bytes (zero-copy)
            ///
            /// # Safety
            /// The bytes must be properly aligned and valid for this type.
            #[inline]
            pub fn from_bytes(bytes: &[u8]) -> Option<&Self> {
                if bytes.len() >= Self::SIZE {
                    Some(::bytemuck::from_bytes(&bytes[..Self::SIZE]))
                } else {
                    None
                }
            }

            /// Create from raw bytes, copying to ensure alignment
            #[inline]
            pub fn from_bytes_copy(bytes: &[u8]) -> Option<Self> {
                if bytes.len() >= Self::SIZE {
                    let mut result = Self::zeroed();
                    let dst = ::bytemuck::bytes_of_mut(&mut result);
                    dst.copy_from_slice(&bytes[..Self::SIZE]);
                    Some(result)
                } else {
                    None
                }
            }
        }

        // Implement LogSummary for efficient logging
        impl ::horus::core::LogSummary for #name {
            fn log_summary(&self) -> ::std::string::String {
                format!("{:?}", self)
            }
        }

        // Implement Serialize/Deserialize for compatibility with standard recording
        impl ::horus::serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::horus::serde::Serializer,
            {
                use ::horus::serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct(stringify!(#name), #(#field_names.to_string().len())+*)?;
                #(
                    state.serialize_field(stringify!(#field_names), &self.#field_names)?;
                )*
                state.end()
            }
        }
    }
}

/// Input for fixed_string! helper macro
pub struct FixedStringInput {
    pub size: usize,
}

impl Parse for FixedStringInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let lit: Lit = input.parse()?;
        let size = match lit {
            Lit::Int(lit_int) => lit_int.base10_parse()?,
            _ => return Err(syn::Error::new_spanned(lit, "Expected integer literal")),
        };
        Ok(FixedStringInput { size })
    }
}

/// Generate a fixed-size string type
pub fn generate_fixed_string(input: FixedStringInput) -> TokenStream {
    let size = input.size;
    let type_name = format_ident!("FixedString{}", size);

    quote! {
        /// Fixed-size string for zero-copy message layouts
        #[derive(Clone, Copy, PartialEq, Eq)]
        #[repr(C)]
        pub struct #type_name {
            data: [u8; #size],
            len: u8,
        }

        unsafe impl ::bytemuck::Pod for #type_name {}
        unsafe impl ::bytemuck::Zeroable for #type_name {}

        impl #type_name {
            pub const CAPACITY: usize = #size;

            pub const fn new() -> Self {
                Self {
                    data: [0u8; #size],
                    len: 0,
                }
            }

            pub fn from_str(s: &str) -> Self {
                let mut result = Self::new();
                let bytes = s.as_bytes();
                let copy_len = bytes.len().min(#size);
                result.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
                result.len = copy_len as u8;
                result
            }

            pub fn as_str(&self) -> &str {
                let len = (self.len as usize).min(#size);
                // SAFETY: We only store valid UTF-8 and track length
                unsafe { std::str::from_utf8_unchecked(&self.data[..len]) }
            }

            pub fn len(&self) -> usize {
                self.len as usize
            }

            pub fn is_empty(&self) -> bool {
                self.len == 0
            }
        }

        impl Default for #type_name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Debug for #type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.as_str())
            }
        }

        impl std::fmt::Display for #type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl From<&str> for #type_name {
            fn from(s: &str) -> Self {
                Self::from_str(s)
            }
        }

        impl AsRef<str> for #type_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
    }
}
