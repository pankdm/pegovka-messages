use std::fs::File;
use std::io::Write;

use image::Rgb;

const FILE_NAME: &str = "input/message2.png";

struct Svg {
    file: File,
}

const ZOOM: usize = 8;
const SHIFT: usize = 2;

#[allow(unused)]
impl Svg {
    pub fn new(width: usize, height: usize) -> Svg {
        let mut file = File::create("output.svg").unwrap();
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

    pub fn add_annotation(&mut self, x: usize, y: usize, dx: usize, dy: usize, value: i32) {
        self.file.write_all(
            format!(
                "<rect x='{}' y='{}' width='{}' height='{}' style='fill:green;opacity:0.5'/>\n",
                x * ZOOM - SHIFT,
                y * ZOOM - SHIFT,
                dx * ZOOM + 2 * SHIFT,
                dy * ZOOM + 2 * SHIFT,
            )
            .as_bytes(),
        );
        let options = "dominant-baseline='middle' text-anchor='middle' fill='white'";
        let style_options = [
            "paint-order: stroke;",
            "fill: white;",
            "stroke: black;",
            "stroke-width: 2px;",
            "font: 20px bold sans;",
        ]
        .join(" ");
        self.file.write_all(
            format!(
                "<text x='{}' y='{}' {} style='{}'>{}</text>\n",
                x * ZOOM + (dx / 2) * ZOOM,
                y * ZOOM + (dy / 2) * ZOOM,
                options,
                style_options,
                value,
            )
            .as_bytes(),
        );
    }
}

fn rgb_to_color(pixel: &Rgb<u8>) -> String {
    match pixel {
        Rgb([0, 0, 0]) => "#333333".to_string(),
        Rgb([255, 255, 255]) => "white".to_string(),
        _ => {
            panic!("Unexpected value: {:?}", pixel);
        }
    }
}

// TODO: unify with the above
fn rgb_to_value(pixel: &Rgb<u8>) -> u8 {
    match pixel {
        Rgb([0, 0, 0]) => 0,
        Rgb([255, 255, 255]) => 1,
        _ => {
            panic!("Unexpected value: {:?}", pixel);
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

enum ParseResult {
    None,
    Integer { dx: usize, dy: usize, value: i32 },
}

fn try_parse_symbol(iw: &ImageWrapper, x: usize, y: usize) -> ParseResult {
    let image = &iw.image;
    if image[x][y] == 0
        && image[x + 1][y] == 1
        && image[x][y + 1] == 1
        && image[x - 1][y] == 0
        && image[x][y - 1] == 0
    {
        // Find proper delta
        let mut delta = 1;
        // 10 as limit should be good enough
        while delta < 10 {
            if x + delta >= iw.width || y + delta >= iw.height {
                break;
            }
            if image[x + delta][y] != 1 || image[x][y + delta] != 1 {
                break;
            }
            delta += 1;
        }

        // Calculate overall value
        let mut value = 0 as i32;
        for cy in 0..(delta - 1) {
            for cx in 0..(delta - 1) {
                if image[x + 1 + cx][y + 1 + cy] == 1 {
                    value += 1 << (cy * (delta - 1) + cx) as i32;
                }
            }
        }

        return ParseResult::Integer {
            dx: delta,
            dy: delta,
            value: value,
        };
    } else {
        return ParseResult::None;
    }
}

fn parse_image(iw: &ImageWrapper, parsed: &mut BooleanGrid, svg: &mut Svg) {
    println!("Parsing image...");
    // skip boundaries
    for y in 1..(iw.height - 2) {
        for x in 1..(iw.width - 2) {
            if parsed[x][y] {
                continue;
            }
            let parse_result = try_parse_symbol(iw, x, y);
            match parse_result {
                ParseResult::None => continue,
                ParseResult::Integer { dx, dy, value } => {
                    println!(
                        "Found Integer at ({}, {}), value = {}, d = ({}, {})",
                        x, y, value, dx, dy
                    );
                    svg.add_annotation(x, y, dx, dy, value);
                }
            }
        }
    }
}

fn main() {
    let img = image::open(FILE_NAME).unwrap().to_rgb();
    println!("Img dimensions: {:?}", img.dimensions());
    let scale = 4;
    let width = img.dimensions().0 / scale;
    let height = img.dimensions().1 / scale;

    let mut svg = Svg::new(width as usize, height as usize);

    // initialize initial data structures
    let mut parsed = Vec::new();
    let mut image = Vec::new();
    for _ in 0..width {
        parsed.push(vec![false; height as usize]);
        image.push(vec![0; height as usize]);
    }

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(scale * x, scale * y);
            image[x as usize][y as usize] = rgb_to_value(pixel);

            // println!("{}, {} -> {:?}", x, y, pixel);
            let color = rgb_to_color(pixel);
            svg.set_pixel(x as usize, y as usize, color);
        }
    }

    let iw = ImageWrapper {
        image: image,
        height: height as usize,
        width: width as usize,
    };

    parse_image(&iw, &mut parsed, &mut svg);

    svg.close()
}
