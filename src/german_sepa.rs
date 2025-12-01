/// Quick and dirty implementation of german(?) payment info strings, purely build on anecdotal
/// evidence.
use nom::{FindSubstring, IResult, Parser, branch::alt, bytes::complete::tag};

#[derive(Default, Debug, PartialEq)]
pub struct PaymentInfo<'a> {
    pub eref: Option<&'a str>,
    pub kref: Option<&'a str>,
    pub mref: Option<&'a str>,
    pub purp: Option<&'a str>,
    pub svwz: Option<&'a str>,
}

impl<'a> PaymentInfo<'a> {
    fn add_tag(&mut self, tag: &str, value: &'a str) {
        match tag {
            "EREF+" => {
                self.eref = Some(value);
            }
            "KREF+" => {
                self.kref = Some(value);
            }
            "MREF+" => {
                self.mref = Some(value);
            }
            "PURP+" => {
                self.purp = Some(value);
            }
            "SVWZ+" => {
                self.svwz = Some(value);
            }
            _ => {}
        }
    }
}

pub fn parse_purpose(input: &str) -> IResult<&str, PaymentInfo<'_>> {
    const TAGS: &[&str] = &["EREF+", "KREF+", "MREF+", "PURP+", "SVWZ+"];
    let mut tag_parser = alt((
        tag("EREF+"),
        tag("KREF+"),
        tag("MREF+"),
        tag("PURP+"),
        tag("SVWZ+"),
    ));

    let mut result = PaymentInfo::default();

    let mut input = input;

    while !input.is_empty() {
        let (rest, tag_name) = tag_parser.parse(input)?;
        input = rest;

        if let Some(offset_till_next_tag) =
            TAGS.iter().filter_map(|t| input.find_substring(t)).min()
        {
            let value = &input[..offset_till_next_tag];
            result.add_tag(tag_name, value);
            input = &input[offset_till_next_tag..];
        } else {
            result.add_tag(tag_name, input);
            input = "";
        }
    }

    Ok((input, result))
}

mod tests {

    #[test]
    fn test_parse_purpose() {
        use crate::german_sepa::{PaymentInfo, parse_purpose};

        let purpose = "EREF+79966714003999040925104641KREF+2025090595383848090700000000043946MREF+OFFLINECRED+DE82CC100000346626PURP+IDCPSVWZ+MH BAECKEREI MEYER GMBH 2198 GIR 79966714/HANNOVER/DE 04.09.2025 um 10:46:41";
        assert_eq!(
            parse_purpose(purpose).unwrap().1,
            PaymentInfo {
                eref: Some("79966714003999040925104641"),
                kref: Some("2025090595383848090700000000043946"),
                mref: Some("OFFLINECRED+DE82CC100000346626"),
                purp: Some("IDCP"),
                svwz: Some(
                    "MH BAECKEREI MEYER GMBH 2198 GIR 79966714/HANNOVER/DE 04.09.2025 um 10:46:41"
                ),
            }
        );

        let purpose = "utdanuodttrondursaduog";
        assert!(parse_purpose(purpose).is_err());
    }
}
