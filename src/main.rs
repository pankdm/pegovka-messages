// Cargo.toml:
// [dependencies]
// image = "0.23.6"
// lazy_static = "1.4.0"
//
// Usage
// decode input file:
//    cargo run input_file output_file
// show all supported symbols:
//    cargo run -- --show-all
// show all encountered symbols from folder:
//    cargo run -- --show-all input_folder

use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;

use image::Rgb;

#[macro_use]
extern crate lazy_static;

const ZOOM: usize = 8;
const SHIFT: usize = 3;

lazy_static! {
    static ref SYMBOLS_LIST: Vec<(i32, &'static str)> = vec![
        (0, "ap"),
        (12, "=="),
        (417, "inc"),
        (401, "dec"),
        (365, "sum"),
        (146, "mul"),
        (40, "div"),
        (448, "eq"),
        (2, "true"),
        (8, "false"),
    ];
    static ref SYMBOLS: HashMap<i32, &'static str> = SYMBOLS_LIST
    .iter()
    .copied()
    .collect();
}

struct Svg {
    file: File,
}

#[allow(unused)]
impl Svg {
    pub fn new(output_file: &String, width: usize, height: usize) -> Svg {
        let mut file = File::create(output_file).unwrap();
        file.write_all(
            format!(
                "<svg xmlns='http://www.w3.org/2000/svg' version='1.1' width='{}' height='{}'>\n",
                width * ZOOM,
                height * ZOOM,
            )
            .as_bytes(),
        );
        file.write_all("<rect width='100%' height='100%' style='fill:black'/>\n".as_bytes());
        Svg { file: file }
    }

    pub fn close(&mut self) {
        self.file.write_all("</svg>".as_bytes());
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: String) {
        self.file.write_all(
            format!(
                "<rect x='{}' y='{}' width='7' height='7' style='fill:{}'/>\n",
                x * ZOOM,
                y * ZOOM,
                color
            )
            .as_bytes(),
        );
    }

    pub fn glyph_to_color(glyph_type: GlyphType) -> String {
        let color = match glyph_type {
            GlyphType::Ineteger => "green",
            GlyphType::Command => "yellow",
            GlyphType::Variable => "blue",
        };
        return color.to_string();
    }

    pub fn add_raw_annotation(
        &mut self,
        x: usize,
        y: usize,
        dx: usize,
        dy: usize,
        text: &String,
        color: &String,
        glyph_type: GlyphType,
    ) {
        self.file.write_all(
            format!(
                "<rect x='{}' y='{}' width='{}' height='{}' style='fill:{};opacity:0.5'/>\n",
                x * ZOOM - SHIFT,
                y * ZOOM - SHIFT,
                dx * ZOOM + 2 * SHIFT,
                dy * ZOOM + 2 * SHIFT,
                color,
            )
            .as_bytes(),
        );
        let options = "dominant-baseline='middle' text-anchor='middle' fill='white'";
        let style_options = [
            "paint-order: stroke;",
            "fill: white;",
            "stroke: black;",
            "stroke-width: 2px;",
            "font: 18px bold sans;",
        ]
        .join(" ");

        self.file.write_all(
            format!(
                "<text x='{}' y='{}' {} style='{}'>{}</text>\n",
                ZOOM * x + ZOOM * dx / 2,
                ZOOM * y + ZOOM * dy / 2,
                options,
                style_options,
                text,
            )
            .as_bytes(),
        );
    }

    pub fn add_annotation(
        &mut self,
        x: usize,
        y: usize,
        dx: usize,
        dy: usize,
        value: i32,
        glyph_type: GlyphType,
        glyph: Glyph,
    ) {
        let text = Svg::annotation_text(glyph_type, glyph);
        let color = Svg::glyph_to_color(glyph_type);
        self.add_raw_annotation(x, y, dx, dy, &text, &color, glyph_type);
    }

    fn annotation_text(glyph_type: GlyphType, glyph: Glyph) -> String {
        match glyph {
            Glyph::Integer(value) => value.to_string(),
            Glyph::Variable(value) => format!("x{}", value),
            Glyph::Command(value) => {
                let text = SYMBOLS.get(&value).unwrap_or(&"");
                return if text == &"" {
                    format!(":{}", value)
                } else {
                    text.to_string()
                };
            }
        }
    }
}

fn value_to_svg_color(value: u8) -> String {
    match value {
        0 => "#333333".to_string(),
        1 => "white".to_string(),
        _ => {
            panic!("Unexpected pixel: {:?}", value);
        }
    }
}

fn rgb_to_value(pixel: &Rgb<u8>) -> u8 {
    match pixel {
        Rgb([0, 0, 0]) => 0,
        Rgb([255, 255, 255]) => 1,
        _ => {
            panic!("Unexpected pixel: {:?}", pixel);
        }
    }
}

type Image = Vec<Vec<u8>>;
type BooleanGrid = Vec<Vec<bool>>;

struct ImageWrapper {
    image: Image,
    height: usize,
    width: usize,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum GlyphType {
    Ineteger,
    Command,
    Variable,
}

#[derive(Clone, Copy)]
enum Glyph {
    Integer(i32),
    Command(i32),
    Variable(i32),
}
type Token = (i32, Glyph);

enum ParseResult {
    None,
    GenericGlyph {
        dx: usize,
        dy: usize,
        value: i32,
        glyph_type: GlyphType,
        glyph: Glyph,
    },
}

// delta boundary is not inclusive
fn is_full_frame(image: &Image, x: usize, y: usize, delta: usize) -> bool {
    for i in 0..delta {
        if image[x][y + i] != 1 {
            return false;
        }
        if image[x + delta - 1][y + 1] != 1 {
            return false;
        }
        if image[x + i][y] != 1 {
            return false;
        }
        if image[x + i][y + delta - 1] != 1 {
            return false;
        }
    }
    true
}

fn try_parse_symbol(iw: &ImageWrapper, x: usize, y: usize, set: u8) -> ParseResult {
    let image = &iw.image;
    if image[x + 1][y] == set
        && image[x][y + 1] == set
        && image[x - 1][y] != set
        && image[x][y - 1] != set
    {
        // control bit
        let control_bit = image[x][y] == set;

        // Find proper delta
        let mut delta = 1;
        // 10 as limit should be good enough
        while delta < 10 {
            if x + delta >= iw.width || y + delta >= iw.height {
                break;
            }
            if image[x + delta][y] != set || image[x][y + delta] != set {
                break;
            }
            delta += 1;
        }

        // Calculate overall value
        let mut final_value = 0 as i32;
        for cy in 0..(delta - 1) {
            for cx in 0..(delta - 1) {
                if image[x + 1 + cx][y + 1 + cy] == set {
                    final_value += 1 << (cy * (delta - 1) + cx) as i32;
                }
            }
        }

        let extra_bit = image[x][y + delta];
        // extra bit indicate negative numbers
        if extra_bit == set {
            final_value = -final_value;
        }
        let (mut glyph_type, mut glyph) = if control_bit {
            (GlyphType::Command, Glyph::Command(final_value))
        } else {
            (GlyphType::Ineteger, Glyph::Integer(final_value))
        };

        // check if it's a variable
        if control_bit && is_full_frame(image, x, y, delta) && set == 1 {
            // println!("Found full frame at ({}, {})", x, y);
            let parse_result = try_parse_symbol(iw, x + 1, y + 1, 0);
            match parse_result {
                ParseResult::None => {
                    println!(
                        "Warning: embedded symbol not recognized at ({}, {}",
                        x + 1,
                        y + 1
                    );
                }
                ParseResult::GenericGlyph {
                    dx: _,
                    dy: _,
                    value,
                    glyph_type: _,
                    ..
                } => {
                    // println!("Found embedded glyph {:?} => variable x{}", glyph_type, value);
                    glyph_type = GlyphType::Variable;
                    glyph = Glyph::Variable(value)
                }
            }
        }

        return ParseResult::GenericGlyph {
            dx: delta,
            dy: delta + extra_bit as usize,
            value: final_value,
            glyph_type: glyph_type,
            glyph: glyph,
        };
    } else {
        return ParseResult::None;
    }
}

fn mark_parsed(parsed: &mut BooleanGrid, x: usize, y: usize, dx: usize, dy: usize) {
    for cdx in 0..dx {
        for cdy in 0..dy {
            parsed[x + cdx][y + cdy] = true;
        }
    }
}

fn parse_image(iw: &ImageWrapper, parsed: &mut BooleanGrid, svg: &mut Svg) -> Vec<Token> {
    let mut codes = Vec::new();
    println!("Parsing image...");
    // skip boundaries
    for y in 1..(iw.height - 2) {
        for x in 1..(iw.width - 2) {
            if parsed[x][y] {
                continue;
            }
            let parse_result = try_parse_symbol(iw, x, y, 1);
            match parse_result {
                ParseResult::None => continue,
                ParseResult::GenericGlyph {
                    dx,
                    dy,
                    value,
                    glyph_type,
                    glyph,
                } => {
                    mark_parsed(parsed, x, y, dx, dy);
                    svg.add_annotation(x, y, dx, dy, value, glyph_type, glyph);
                    if glyph_type == GlyphType::Command || glyph_type == GlyphType::Variable {
                        codes.push((value, glyph));
                    }
                }
            }
        }
    }
    println!("Done");
    codes
}

fn create_empty_image(width: usize, height: usize) -> Image {
    let mut image = Vec::new();
    for _ in 0..width {
        image.push(vec![0; height]);
    }
    image
}

fn encode_symbol(value: i32) -> Image {
    assert!(value >= 0);
    // println!("Encoding {}", value);
    let ln = if value == 0 {
        1.0
    } else {
        ((value + 1) as f32).log2().ceil()
    };
    let d = ln.sqrt().ceil() as usize;
    // println!("  d = {}", d);
    let mut image = create_empty_image(d + 1, d + 1);
    for i in 0..=d {
        image[i][0] = 1;
        image[0][i] = 1;
    }

    for cy in 0..d {
        for cx in 0..d {
            let bit = 1 << (cx + cy * d);
            // println!("    checking against {}", bit);
            if (value & bit) > 0 {
                image[1 + cx][1 + cy] = 1;
            }
        }
    }

    image
}

fn show_symbols(tokens: Vec<Token>, output_file: &String) {
    let mut images = Vec::new();
    let mut max_dx = 0;
    let offset = 2;
    let mut total_dy = offset;
    for token in tokens.iter() {
        let image = encode_symbol(token.0);
        max_dx = max_dx.max(image.len());
        total_dy += image[0].len() + 4;
        images.push(image);
    }

    let svg_width = 4 * (max_dx + 4) as usize;
    let svg_height = total_dy as usize;
    let mut svg = Svg::new(&output_file, svg_width, svg_height);
    // initally set all pixels to black
    for x in 0..svg_width {
        for y in 0..svg_height {
            svg.set_pixel(x, y, value_to_svg_color(0));
        }
    }

    let mut y0 = offset;
    for index in 0..images.len() {
        let image = &images[index];
        for repeat in 0..4 {
            let x0 = offset + (max_dx + 4) * repeat;
            let glyph = tokens[index].1;
            let glyph_type = match glyph {
                Glyph::Variable(_) => GlyphType::Variable,
                _ if repeat == 3 => {
                    continue;
                }
                _ => GlyphType::Command,
            };

            for dx in 0..image.len() {
                for dy in 0..image[0].len() {
                    if image[dx][dy] == 1 {
                        svg.set_pixel(x0 + dx, y0 + dy, value_to_svg_color(1));
                    }
                }
            }

            if repeat == 1 {
                let text = format!("{}", tokens[index].0);
                svg.add_raw_annotation(
                    x0,
                    y0,
                    image.len(),
                    image[0].len(),
                    &text,
                    &"yellow".to_string(),
                    GlyphType::Command,
                );
            }
            if repeat == 2 {
                svg.add_annotation(
                    x0,
                    y0,
                    image.len(),
                    image[0].len(),
                    tokens[index].0,
                    glyph_type,
                    glyph,
                );
            }
            if repeat == 3 {
                match glyph {
                    Glyph::Variable(value) => {
                        // let text = format!("{}", value);
                        let text = format!("!{}", value);
                        // shift initial position to mark embedded Int
                        svg.add_raw_annotation(
                            x0 + 1,
                            y0 + 1,
                            image.len() - 2,
                            image[0].len() - 2,
                            &text,
                            &"green".to_string(),
                            GlyphType::Ineteger,
                        );
                    }
                    _ => {}
                }
            }
        }
        y0 += image[0].len() + 4;
    }

    svg.close();
}

fn show_all_symbols_from_dict() {
    let mut tokens = Vec::new();
    for (code, _) in SYMBOLS_LIST.iter() {
        let glyph = Glyph::Command(*code);
        tokens.push((*code, glyph));
    }
    show_symbols(tokens, &"glyphs-dict.svg".to_string());
}

pub fn split_string(s: &String, pattern: &str) -> Vec<String> {
    let mut res = Vec::new();
    for part in s.split(pattern) {
        res.push(part.to_string());
    }
    return res;
}

fn get_default_output_file(input_file: &String) -> String {
    assert_eq!(input_file.ends_with(".png"), true);
    let parts = split_string(&input_file, "/");
    let file_name = parts.last().unwrap();
    let new_file_name = file_name.replace(".png", ".svg");
    let output_file = format!("output/{}", new_file_name);
    output_file
}

fn show_all_symbols_from_folder(folder: &String) {
    let mut unique = HashSet::new();
    let mut all_tokens = Vec::new();

    let paths = fs::read_dir(folder).unwrap();
    for path in paths {
        let full_path = path.unwrap().path();
        // println!("Found: {}", &full_path.display());
        let input_file = full_path.to_str().unwrap().to_string();
        if input_file.ends_with(".png") {
            let output_file = get_default_output_file(&input_file);
            let tokens = parse_file(&input_file, &output_file);

            for (code, glyph) in tokens.iter() {
                if !unique.contains(code) {
                    unique.insert(*code);
                    all_tokens.push((*code, *glyph));
                }
            }
        }
    }

    all_tokens.sort_by(|a, b| {
        match (a.1, b.1) {
            (Glyph::Variable(va), Glyph::Variable(vb)) => {
                va.partial_cmp(&vb).unwrap()
            },
            _ => a.0.partial_cmp(&b.0).unwrap()
        }
    });

    show_symbols(all_tokens, &"glyphs-all.svg".to_string());
}

fn parse_file(input_file: &String, output_file: &String) -> Vec<Token> {
    println!("Processing {}, output -> {}", &input_file, &output_file);
    let img = image::open(&input_file).unwrap().to_rgb();
    println!("  Img dimensions: {:?}", img.dimensions());
    let scale = 4;
    let width = img.dimensions().0 / scale;
    let height = img.dimensions().1 / scale;

    let mut svg = Svg::new(&output_file, width as usize, height as usize);

    // initialize empty data structures
    let mut parsed = Vec::new();
    let mut image = create_empty_image(width as usize, height as usize);
    for _ in 0..width {
        parsed.push(vec![false; height as usize]);
    }

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(scale * x, scale * y);
            let value = rgb_to_value(pixel);
            image[x as usize][y as usize] = value;

            let color = value_to_svg_color(value);
            svg.set_pixel(x as usize, y as usize, color);
        }
    }

    let iw = ImageWrapper {
        image: image,
        height: height as usize,
        width: width as usize,
    };

    let tokens = parse_image(&iw, &mut parsed, &mut svg);
    svg.close();
    tokens
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("Running {:?}, len = {}", args, args.len());
    assert!(args.len() >= 2);
    if args[1] == "--show-all" {
        if args.len() >= 3 {
            show_all_symbols_from_folder(&args[2].to_string());
        } else {
            show_all_symbols_from_dict();
        }
        return;
    }

    let input_file = args[1].to_string();
    let output_file = if args.len() >= 3 {
        args[2].to_string()
    } else {
        get_default_output_file(&input_file)
    };

    parse_file(&input_file, &output_file);
}
