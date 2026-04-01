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

impl InstKind {
    /// True for I-type memory loads (base+offset addressing in disassembly).
    pub fn is_load(self) -> bool {
        matches!(
            self,
            Self::lb
                | Self::lh
                | Self::lw
                | Self::ld
                | Self::lbu
                | Self::lhu
                | Self::lwu
                | Self::flw
                | Self::fld
        )
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum InstFormat {
    R,
    FR,
    FR4,
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
            Self::R | Self::FR | Self::FR4 | Self::I | Self::S | Self::B | Self::U | Self::J
        )
    }
}

impl std::str::FromStr for InstFormat {
    type Err = crate::XError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(Self::R),
            "FR" => Ok(Self::FR),
            "FR4" => Ok(Self::FR4),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inst_kind_from_name_and_roundtrip() {
        for (name, expected) in [
            ("add", InstKind::add),
            ("jal", InstKind::jal),
            ("c_addi", InstKind::c_addi),
        ] {
            let kind = InstKind::from_name(name).unwrap();
            assert_eq!(kind, expected);
            assert_eq!(InstKind::from_name(kind.as_str()).unwrap(), kind);
        }
        assert!(matches!(
            InstKind::from_name("nonexistent"),
            Err(XError::ParseError)
        ));
    }

    #[test]
    fn inst_format_is_compressed() {
        assert!(!InstFormat::R.is_compressed());
        assert!(!InstFormat::I.is_compressed());
        assert!(!InstFormat::S.is_compressed());
        assert!(!InstFormat::B.is_compressed());
        assert!(!InstFormat::U.is_compressed());
        assert!(!InstFormat::J.is_compressed());

        assert!(InstFormat::CR.is_compressed());
        assert!(InstFormat::CI.is_compressed());
        assert!(InstFormat::CSS.is_compressed());
        assert!(InstFormat::CIW.is_compressed());
        assert!(InstFormat::CL.is_compressed());
        assert!(InstFormat::CS.is_compressed());
        assert!(InstFormat::CA.is_compressed());
        assert!(InstFormat::CB.is_compressed());
        assert!(InstFormat::CJ.is_compressed());
    }

    #[test]
    fn inst_format_from_str() {
        assert!(matches!("R".parse::<InstFormat>().unwrap(), InstFormat::R));
        assert!(matches!(
            "CI".parse::<InstFormat>().unwrap(),
            InstFormat::CI
        ));
        assert!("X".parse::<InstFormat>().is_err());
        assert!("".parse::<InstFormat>().is_err());
    }
}
