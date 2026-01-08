use anyhow::{Result, anyhow};
use nix::unistd::{Group, User};
use std::fs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerFilter {
    uid: Check<u32>,
    gid: Check<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Check<T> {
    Equal(T),
    NotEq(T),
    Ignore,
}

impl OwnerFilter {
    const IGNORE: Self = Self {
        uid: Check::Ignore,
        gid: Check::Ignore,
    };

    /// Parses an owner constraint
    /// Returns an error if the string is invalid
    /// Returns Ok(None) when string is acceptable but a noop (such as "" or ":")
    pub fn from_string(input: &str) -> Result<Self> {
        let mut it = input.split(':');
        let (fst, snd) = (it.next(), it.next());

        if it.next().is_some() {
            return Err(anyhow!(
                "more than one ':' present in owner string '{}'. See 'fd --help'.",
                input
            ));
        }

        let uid = Check::parse(fst, |s| {
            if let Ok(uid) = s.parse() {
                Ok(uid)
            } else {
                User::from_name(s)?
                    .map(|user| user.uid.as_raw())
                    .ok_or_else(|| anyhow!("'{}' is not a recognized user name", s))
            }
        })?;
        let gid = Check::parse(snd, |s| {
            if let Ok(gid) = s.parse() {
                Ok(gid)
            } else {
                Group::from_name(s)?
                    .map(|group| group.gid.as_raw())
                    .ok_or_else(|| anyhow!("'{}' is not a recognized group name", s))
            }
        })?;

        Ok(Self { uid, gid })
    }

    /// If self is a no-op (ignore both uid and gid) then return `None`, otherwise wrap in a `Some`
    pub fn filter_ignore(self) -> Option<Self> {
        if self == Self::IGNORE {
            None
        } else {
            Some(self)
        }
    }

    pub fn matches(&self, md: &fs::Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;

        self.uid.check(md.uid()) && self.gid.check(md.gid())
    }
}

impl<T: PartialEq> Check<T> {
    fn check(&self, v: T) -> bool {
        match self {
            Self::Equal(x) => v == *x,
            Self::NotEq(x) => v != *x,
            Self::Ignore => true,
        }
    }

    fn parse<F>(s: Option<&str>, f: F) -> Result<Self>
    where
        F: Fn(&str) -> Result<T>,
    {
        let (s, equality) = match s {
            Some("") | None => return Ok(Self::Ignore),
            Some(s) if s.starts_with('!') => (&s[1..], false),
            Some(s) => (s, true),
        };

        f(s).map(|x| {
            if equality {
                Self::Equal(x)
            } else {
                Self::NotEq(x)
            }
        })
    }
}

#[cfg(test)]
mod owner_parsing {
    use super::OwnerFilter;

    macro_rules! owner_tests {
        ($($name:ident: $value:expr => $result:pat,)*) => {
            $(
                #[test]
                fn $name() {
                    let o = OwnerFilter::from_string($value);
                    match o {
                        $result => {},
                        _ => panic!("{:?} does not match {}", o, stringify!($result)),
                    }
                }
            )*
        };
    }

    use super::Check::*;
    owner_tests! {
        empty:      ""      => Ok(OwnerFilter::IGNORE),
        uid_only:   "5"     => Ok(OwnerFilter { uid: Equal(5), gid: Ignore     }),
        uid_gid:    "9:3"   => Ok(OwnerFilter { uid: Equal(9), gid: Equal(3)   }),
        gid_only:   ":8"    => Ok(OwnerFilter { uid: Ignore,   gid: Equal(8)   }),
        colon_only: ":"     => Ok(OwnerFilter::IGNORE),
        trailing:   "5:"    => Ok(OwnerFilter { uid: Equal(5), gid: Ignore     }),

        uid_negate: "!5"    => Ok(OwnerFilter { uid: NotEq(5), gid: Ignore     }),
        both_negate:"!4:!3" => Ok(OwnerFilter { uid: NotEq(4), gid: NotEq(3)   }),
        uid_not_gid:"6:!8"  => Ok(OwnerFilter { uid: Equal(6), gid: NotEq(8)   }),

        more_colons:"3:5:"  => Err(_),
        only_colons:"::"    => Err(_),
    }
}
