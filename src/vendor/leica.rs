use crate::tiff::{tag, Data::Ascii, Tiff};
use regex::Regex;
use roxmltree;

pub(crate) fn is_compatible(tiff: &Tiff) -> bool {
    let num_ifd = tiff.len();
    match tiff.get(0) {
        Some(d) => {
            let is_tile = d.contains_key(&tag::TileOffsets);
            let img_desc = d.contains_key(&tag::ImageDescription);
            let img_desc_str = d.get(&tag::ImageDescription);
            let has_valid_leica_xml = match d.get(&tag::ImageDescription) {
                Some(Ascii(l)) => check_leica_xml(l),
                _ => false,
            };
            // println!("ImgDesc: {:?}", img_desc_str);
            println!("Num {:?}", num_ifd);
            is_tile && has_valid_leica_xml
        }
        None => false,
    }
}

const LEICA_XMLNS_1: &str = "http://www.leica-microsystems.com/scn/2010/03/10";
const LEICA_XMLNS_2: &str = "http://www.leica-microsystems.com/scn/2010/10/01";

fn check_leica_xml(l: &String) -> bool {
    println!("{}", l);
    let re1 = Regex::new(LEICA_XMLNS_1).unwrap();
    let re2 = Regex::new(LEICA_XMLNS_2).unwrap();

    println!("Check xml {:?}", re1.is_match(l));
    println!("Check xml {:?}", re2.is_match(l));

    let re3 = Regex::new("\0$").unwrap();
    let l2 = re3.replace(l, "");
    let docs = roxmltree::Document::parse(&l2);
    let _ = parser_leica_xml(l);
    // println!("Check xml3 {:?}", docs);
    re1.is_match(l) || re2.is_match(l)
}

struct LeicaSpec {}

struct ImageSpec {}

fn parser_leica_xml<'a>(l: &'a String) -> Result<LeicaSpec, roxmltree::Error> {
    let re3 = Regex::new("\0$").unwrap();
    let l = re3.replace(l, "");

    let docs = roxmltree::Document::parse(&l)?;
    let root = docs.root();
    println!("Root Tag name: {:?}", root.tag_name());

    let scn = root.first_child().unwrap();
    let collection = scn.first_child().unwrap();
    println!("SCN Tag name: {:?}", scn.tag_name().name());

    Ok(LeicaSpec {})
}

fn parse_image_tag() {
    todo!()
}

const IMG_XML: &str = r#"
    <image name="sample_name" uuid="urn:uuid:00000000-0000-0000-0000-000000000001">
      <creationDate>2099-09-09T09:43:06.873Z</creationDate>
      <device model="Leica SCN400;Leica SCN" version="1.4.0.9691 2011/03/30 10:30:59;1.4.0.9708" />
      <pixels sizeX="36832" sizeY="38432">
        <dimension sizeX="36832" sizeY="38432" r="0" ifd="3" />
        <dimension sizeX="9208" sizeY="9608" r="1" ifd="4" />
      </pixels>
      <view sizeX="18416000" sizeY="19216000" offsetX="5389341" offsetY="17548313" spacingZ="400" />
      <scanSettings>
        <objectiveSettings>
          <objective>20</objective>
        </objectiveSettings>
        <illuminationSettings>
          <numericalAperture>0.4</numericalAperture>
          <illuminationSource>brightfield</illuminationSource>
        </illuminationSettings>
      </scanSettings>
    </image>
"#;
