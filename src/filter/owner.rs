use anyhow::{anyhow, Result};
use std::fs;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OwnerFilter {
    uid: Check<u32>,
    gid: Check<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Check<T> {
    Equal(T),
    NotEq(T),
    Ignore,
}

impl OwnerFilter {
    pub fn from_string(input: &str) -> Result<Self> {
        let mut it = input.split(':');
        let (fst, snd) = (it.next(), it.next());

        let uid = Check::parse(fst, |s| {
            s.parse()
                .ok()
                .or_else(|| users::get_user_by_name(s).map(|user| user.uid()))
                .ok_or_else(|| anyhow!("'{}' is not a recognized user name", s))
        })?;
        let gid = Check::parse(snd, |s| {
            s.parse()
                .ok()
                .or_else(|| users::get_group_by_name(s).map(|group| group.gid()))
                .ok_or_else(|| anyhow!("'{}' is not a recognized group name", s))
        })?;

        if let (Check::Ignore, Check::Ignore) = (uid, gid) {
            Err(anyhow!(
                "'{}' is not a valid user/group specifier. See 'fd --help'.",
                input
            ))
        } else {
            Ok(OwnerFilter { uid, gid })
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
            Check::Equal(x) => v == *x,
            Check::NotEq(x) => v != *x,
            Check::Ignore => true,
        }
    }

    fn parse<F>(s: Option<&str>, f: F) -> Result<Self>
    where
        F: Fn(&str) -> Result<T>,
    {
        let (s, equality) = match s {
            Some("") | None => return Ok(Check::Ignore),
            Some(s) if s.starts_with('!') => (&s[1..], false),
            Some(s) => (s, true),
        };

        f(s).map(|x| {
            if equality {
                Check::Equal(x)
            } else {
                Check::NotEq(x)
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
        empty:      ""      => Err(_),
        uid_only:   "5"     => Ok(OwnerFilter { uid: Equal(5), gid: Ignore    }),
        uid_gid:    "9:3"   => Ok(OwnerFilter { uid: Equal(9), gid: Equal(3)  }),
        gid_only:   ":8"    => Ok(OwnerFilter { uid: Ignore,   gid: Equal(8)  }),
        colon_only: ":"     => Err(_),
        trailing:   "5:"    => Ok(OwnerFilter { uid: Equal(5), gid: Ignore    }),

        uid_negate: "!5"    => Ok(OwnerFilter { uid: NotEq(5), gid: Ignore    }),
        both_negate:"!4:!3" => Ok(OwnerFilter { uid: NotEq(4), gid: NotEq(3)  }),
        uid_not_gid:"6:!8"  => Ok(OwnerFilter { uid: Equal(6), gid: NotEq(8)  }),
    }
}
