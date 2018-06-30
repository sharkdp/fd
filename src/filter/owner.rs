use anyhow::{anyhow, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OwnerFilter {
    uid: Option<u32>,
    gid: Option<u32>,
}

impl OwnerFilter {
    pub fn from_string(input: &str) -> Result<Self> {
        let mut it = input.split(':');
        let (fst, snd) = (it.next(), it.next());

        let uid = match fst {
            Some("") | None => None,
            Some(s) => Some(s.parse()?),
        };
        let gid = match snd {
            Some("") | None => None,
            Some(s) => Some(s.parse()?),
        };

        if uid.is_none() && gid.is_none() {
            Err(anyhow!(
                "'{}' is not a valid user/group specifier. See 'fd --help'.",
                input
            ))
        } else {
            Ok(OwnerFilter { uid, gid })
        }
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

    owner_tests! {
        empty:      ""      => Err(_),
        uid_only:   "5"     => Ok(OwnerFilter { uid: Some(5), gid: None   }),
        uid_gid:    "9:3"   => Ok(OwnerFilter { uid: Some(9), gid: Some(3)}),
        gid_only:   ":8"    => Ok(OwnerFilter { uid: None,    gid: Some(8)}),
        colon_only: ":"     => Err(_),
        trailing:   "5:"    => Ok(OwnerFilter { uid: Some(5), gid: None   }),
    }
}
