use std::io;

use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{char, digit1, line_ending, not_line_ending, space0, space1};
use nom::combinator::{complete, cut, map, map_res, opt, recognize, value};
use nom::multi::{many0, many1, many_m_n};
use nom::sequence::{delimited, pair, preceded, terminated, tuple};
use nom::Parser;

use crate::common::*;
use nom::branch::alt;
use std::str::FromStr;

type NomError<'a> = nom::error::VerboseError<&'a str>;
type NomResult<'a, O> = nom::IResult<&'a str, O, NomError<'a>>;

fn end_of_line_comment(input: &str) -> NomResult<Option<&str>> {
    terminated(
        opt(preceded(pair(space0, char('#')), not_line_ending)),
        line_ending,
    )(input)
}

// parse comments, new lines, until next actual token
fn eol(input: &str) -> NomResult<()> {
    value((), pair(many1(end_of_line_comment), space0))(input)
}

fn is_not_eol(input: &str) -> NomResult<&str> {
    is_not("#\r\n")(input)
}

fn word(input: &str) -> NomResult<String> {
    map(is_not(" #\r\n"), String::from)(input)
}

fn uint<T: FromStr>(input: &str) -> NomResult<T> {
    map_res(terminated(digit1, space0), T::from_str)(input)
}

fn sint<T: FromStr>(input: &str) -> NomResult<T> {
    terminated(
        map_res(recognize(pair(opt(char('-')), digit1)), T::from_str),
        space0,
    )(input)
}

fn opt_uint<T: FromStr>(input: &str) -> NomResult<Option<T>> {
    alt((
        map(terminated(tag("-1"), space0), |_| None),
        map(uint, Some),
    ))(input)
}

fn float<T: FromStr>(input: &str) -> NomResult<T> {
    terminated(
        map_res(
            recognize(tuple((opt(char('-')), digit1, char('.'), digit1))),
            T::from_str,
        ),
        space0,
    )(input)
}

// handle the TAG <ws> DATA <eol> format used everywhere
fn entry<'a, V>(
    tag_name: &'static str,
    value: impl Parser<&'a str, V, NomError<'a>>,
) -> impl FnMut(&'a str) -> NomResult<'a, V> {
    terminated(entry_inline(tag_name, value), eol)
}

fn entry_opt<'a, V>(
    tag_name: &'static str,
    value: impl Parser<&'a str, V, NomError<'a>>,
) -> impl FnMut(&'a str) -> NomResult<'a, Option<V>> {
    terminated(
        preceded(tag(tag_name), opt(delimited(space1, cut(value), space0))),
        eol,
    )
}

// handle the TAG <ws> DATA inline
fn entry_inline<'a, V>(
    tag_name: &'static str,
    value: impl Parser<&'a str, V, NomError<'a>>,
) -> impl FnMut(&'a str) -> NomResult<'a, V> {
    preceded(pair(tag(tag_name), space1), terminated(cut(value), space0))
}

pub struct Lev {
    pub palette_name: String,
    pub parallax: Vec2f32,
    pub texture_names: Vec<String>,
    pub sectors: Vec<Sector>,
}

impl Lev {
    pub fn read(mut file: impl io::Read) -> io::Result<Self> {
        let mut str = String::new();
        file.read_to_string(&mut str)?;
        let input: &str = &str;
        let (_, result) =
            complete(Self::parse)(input).map_err(|err: nom::Err<NomError>| match err {
                nom::Err::Incomplete(_) => unreachable!(),
                nom::Err::Error(error) | nom::Err::Failure(error) => {
                    let message = nom::error::convert_error(input, error);
                    eprintln!("{}", message);
                    io::Error::new(io::ErrorKind::InvalidData, message)
                }
            })?;
        Ok(result)
    }

    pub fn parse(input: &str) -> NomResult<Self> {
        let (input, _version) = entry("LEV", is_not_eol)(input)?;
        let (input, _name) = entry("LEVELNAME", is_not_eol)(input)?;
        let (input, palette_name) = entry("PALETTE", word)(input)?;
        let (input, _music) = entry("MUSIC", is_not_eol)(input)?;
        let (input, parallax) = entry(
            "PARALLAX",
            map(pair(float, float), |(x, y)| Vec2f32 { x, y }),
        )(input)?;
        let (input, _texture_count) = entry("TEXTURES", is_not_eol)(input)?;
        let (input, texture_names) = many0(entry("TEXTURE:", word))(input)?;

        let (input, _sector_count) = entry("NUMSECTORS", is_not_eol)(input)?;

        // dbg! { version, name, palette, music, parallax, texture_count, textures, sector_count };

        let (input, sectors) = many0(Sector::parse)(input)?;

        let result = Self {
            palette_name,
            parallax,
            texture_names,
            sectors,
        };

        Ok((input, result))
    }
}

pub struct Sector {
    pub id: u32,
    pub name: Option<String>,
    pub ambient: u32,
    pub floor_texture: Texture,
    pub floor_altitude: f32,
    pub ceiling_texture: Texture,
    pub ceiling_altitude: f32,
    pub second_altitude: f32,
    pub flags: (u32, u32, u32),
    pub layer: i32,
    pub vertices: Vec<Vec2f32>,
    pub walls: Vec<Wall>,
}

impl Sector {
    fn parse(input: &str) -> NomResult<Self> {
        let (input, id) = entry("SECTOR", uint)(input)?;
        let (input, name) = entry_opt("NAME", word)(input)?;
        let (input, ambient) = entry("AMBIENT", uint)(input)?;
        let (input, floor_texture) = entry("FLOOR TEXTURE", Texture::parse)(input)?;
        let (input, floor_altitude) = entry("FLOOR ALTITUDE", float)(input)?;
        let (input, ceiling_texture) = entry("CEILING TEXTURE", Texture::parse)(input)?;
        let (input, ceiling_altitude) = entry("CEILING ALTITUDE", float)(input)?;
        let (input, second_altitude) = entry("SECOND ALTITUDE", float)(input)?;
        let (input, flags) = entry("FLAGS", tuple((uint, uint, uint)))(input)?;
        let (input, layer) = entry("LAYER", sint)(input)?;

        let (input, vertex_count) = entry("VERTICES", uint)(input)?;
        let (input, vertices) = many_m_n(
            vertex_count,
            vertex_count,
            terminated(
                map(
                    pair(entry_inline("X:", float), entry_inline("Z:", float)),
                    |(x, y)| Vec2f32 { x, y },
                ),
                eol,
            ),
        )(input)?;

        let (input, wall_count) = entry("WALLS", uint)(input)?;
        let (input, walls) =
            many_m_n(wall_count, wall_count, entry_inline("WALL", Wall::parse))(input)?;

        let result = Self {
            id,
            name,
            ambient,
            floor_texture,
            floor_altitude,
            ceiling_texture,
            ceiling_altitude,
            second_altitude,
            flags,
            layer,
            vertices,
            walls,
        };

        Ok((input, result))
    }
}

pub struct Wall {
    pub left_vertex: usize,
    pub right_vertex: usize,
    pub middle_texture: Texture,
    pub top_texture: Texture,
    pub bottom_texture: Texture,
    pub sign_texture: Texture,
    pub adjoin_sector: Option<usize>,
    pub mirror_wall: Option<usize>,
    pub walk_sector: Option<usize>,
    pub flags: (u32, u32, u32),
    pub light: u32,
}

impl Wall {
    fn parse(input: &str) -> NomResult<Self> {
        let (input, left_vertex) = entry_inline("LEFT:", uint)(input)?;
        let (input, right_vertex) = entry_inline("RIGHT:", uint)(input)?;
        let (input, middle_texture) = entry_inline("MID:", Texture::parse)(input)?;
        let (input, top_texture) = entry_inline("TOP:", Texture::parse)(input)?;
        let (input, bottom_texture) = entry_inline("BOT:", Texture::parse)(input)?;
        let (input, sign_texture) = entry_inline("SIGN:", Texture::parse_no_flag)(input)?;
        let (input, adjoin_sector) = entry_inline("ADJOIN:", opt_uint)(input)?;
        let (input, mirror_wall) = entry_inline("MIRROR:", opt_uint)(input)?;
        let (input, walk_sector) = entry_inline("WALK:", opt_uint)(input)?;
        let (input, flags) = entry_inline("FLAGS:", tuple((uint, uint, uint)))(input)?;
        let (input, light) = entry_inline("LIGHT:", uint)(input)?;
        let (input, _) = eol(input)?;

        let result = Self {
            left_vertex,
            right_vertex,
            middle_texture,
            top_texture,
            bottom_texture,
            sign_texture,
            adjoin_sector,
            mirror_wall,
            walk_sector,
            flags,
            light,
        };

        Ok((input, result))
    }
}

pub struct Texture {
    pub index: Option<usize>,
    pub offset: Vec2f32,
}

impl Texture {
    fn parse(input: &str) -> NomResult<Self> {
        let (input, result) = Self::parse_no_flag(input)?;
        let (input, _) = uint::<u32>(input)?;
        Ok((input, result))
    }

    fn parse_no_flag(input: &str) -> NomResult<Self> {
        let (input, index) = opt_uint(input)?;
        let (input, x) = float(input)?;
        let (input, y) = float(input)?;
        let result = Self {
            index,
            offset: Vec2f32 { x, y },
        };
        Ok((input, result))
    }
}
