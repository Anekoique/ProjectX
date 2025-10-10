macro_rules! define_inst_kind {
    ( $( ($fmt:ident, ($($arg:ident),*), [$($name:ident),*]) ),* $(,)? ) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum InstKind {
            $( $( $name ),* ),*
        }

        impl InstKind {
            #[inline]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $( $( stringify!($name) => Some(Self::$name), )* )*
                    _ => None,
                }
            }

            #[inline]
            pub fn as_str(self) -> &'static str {
                match self {
                    $( $( Self::$name => stringify!($name), )* )*
                }
            }
        }
    };
}

crate::rv_inst_table!(define_inst_kind);

#[derive(Debug, Clone, Copy)]
pub enum InstFormat {
    R,
    I,
    S,
    B,
    U,
    J,
}

impl std::str::FromStr for InstFormat {
    type Err = crate::XError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(Self::R),
            "I" => Ok(Self::I),
            "S" => Ok(Self::S),
            "B" => Ok(Self::B),
            "U" => Ok(Self::U),
            "J" => Ok(Self::J),
            _ => Err(crate::XError::ParseError),
        }
    }
}
