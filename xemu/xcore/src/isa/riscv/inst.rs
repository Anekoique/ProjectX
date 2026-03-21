use crate::{XError, XResult};

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
            pub fn from_name(name: &str) -> XResult<Self> {
                match name {
                    $( $( stringify!($name) => Ok(Self::$name), )* )*
                    _ => Err(XError::ParseError),
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
#[allow(clippy::upper_case_acronyms)]
pub enum InstFormat {
    R,
    I,
    S,
    B,
    U,
    J,
    CR,
    CI,
    CSS,
    CIW,
    CL,
    CS,
    CA,
    CB,
    CJ,
}

impl InstFormat {
    #[inline]
    pub fn is_compressed(self) -> bool {
        !matches!(
            self,
            Self::R | Self::I | Self::S | Self::B | Self::U | Self::J
        )
    }
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
            "CR" => Ok(Self::CR),
            "CI" => Ok(Self::CI),
            "CSS" => Ok(Self::CSS),
            "CIW" => Ok(Self::CIW),
            "CL" => Ok(Self::CL),
            "CS" => Ok(Self::CS),
            "CA" => Ok(Self::CA),
            "CB" => Ok(Self::CB),
            "CJ" => Ok(Self::CJ),
            _ => Err(crate::XError::ParseError),
        }
    }
}

