use image::{DynamicImage, Rgba, GenericImageView};

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub chroma: u8,
    pub luminance: f32,
}

impl Color {
    pub fn from_rgba(pixel: Rgba<u8>) -> Self {
        let [r, g, b, _a] = pixel.0;
        let chroma = r.max(g).max(b) - r.min(g).min(b);
        let luminance = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
        
        Self {
            r,
            g,
            b,
            chroma,
            luminance,
        }
    }

    pub fn distance_to(&self, other: &Self) -> f32 {
        let dr = self.r as i32 - other.r as i32;
        let dg = self.g as i32 - other.g as i32;
        let db = self.b as i32 - other.b as i32;
        
        return ((dr * dr + dg * dg + db * db) as f32).sqrt()
    }

    pub fn with_saturation(mut self, saturation: f32) -> Self {
        if saturation == 1.0 || self.chroma == 0 { return self; }
        
        let gray = (self.luminance * 255.0) as f32;

        self.r = (gray + (self.r as f32 - gray) * saturation).clamp(0.0, 255.0) as u8;
        self.g = (gray + (self.g as f32 - gray) * saturation).clamp(0.0, 255.0) as u8;
        self.b = (gray + (self.b as f32 - gray) * saturation).clamp(0.0, 255.0) as u8;
        
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        self.chroma = max - min;
        self.luminance = (0.2126 * self.r as f32 + 0.7152 * self.g as f32 + 0.0722 * self.b as f32) / 255.0;
        
        return self
    }
}

pub struct Colorscheme {
    pub palette: Vec<Color>,
    pub background: Color, 
    pub foreground: Color,
}

impl Colorscheme {
    pub fn with_saturation(self, saturation: f32) -> Self {
        Self {
            palette: self.palette
                    .into_iter()
                    .map(|c| c.with_saturation(saturation))
                    .collect(),
            background: self.background.with_saturation(saturation),
            foreground: self.foreground.with_saturation(saturation),
        }
    }
}

pub fn sample_4by4_area(img: &DynamicImage, x: usize, y: usize, w: usize, h: usize) -> Option<Color> {
    if x + 3 >= w || y + 3 >= h { return None }

    let mut r_sum: u32 = 0;
    let mut g_sum: u32 = 0;
    let mut b_sum: u32 = 0;
    let mut count: u32 = 0;

    for ky in 0..4u32 {
        for kx in 0..4u32 {
            let pixel = img.get_pixel(x as u32 + kx , y as u32 + ky );
            let [r, g, b, a] = pixel.0;

            if a == 0 { continue; }
            
            r_sum += r as u32;
            g_sum += g as u32;
            b_sum += b as u32;
            count += 1;
        }
    }
    if count == 0 { return None }

    return Some(Color::from_rgba(Rgba([
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
        255,
    ])))
}

pub fn aaverage_generate_colorscheme(img: &DynamicImage) -> Colorscheme {
    const DIVISOR:       usize = 32;
    const SAMPLE_COUNT:  usize = 1024;
    const PALETTE_COUNT: usize = 16;
    
    let w = img.width() as usize;
    let h = img.height() as usize;
    let mut samples: Vec<Color> = Vec::with_capacity(SAMPLE_COUNT);
    
    let step_x = (w / DIVISOR).max(1);
    let step_y = (h / DIVISOR).max(1);
    let mut darkest  = Color {r: 255, g: 255, b: 255, chroma: 0, luminance: 1.0};
    let mut lightest = Color {r: 0, g: 0, b: 0, chroma: 0, luminance: 0.0};
    
    'pixels: for y in (0..h).step_by(step_y) {
        for x in (0..w).step_by(step_x) {
            if samples.len() >= SAMPLE_COUNT {
                break 'pixels;
            }
           
            let Some(c) = sample_4by4_area(img, x, y, w, h) else {
                continue;
            };
            if c.luminance < darkest.luminance && c.luminance > 0.05  { darkest = c };
            if c.luminance > lightest.luminance && c.luminance < 0.95 { lightest = c };

            samples.push(c);
        }
    }
    samples.sort_unstable_by(|a, b| b.chroma.cmp(&a.chroma));
    
    let mut palette: Vec<Color> = Vec::with_capacity(SAMPLE_COUNT);
    for sample in &samples {
        let diff_bg = (sample.luminance - darkest.luminance).abs(); 
        let diff_fg = (sample.luminance - lightest.luminance).abs(); 
        if diff_bg < 0.08 || diff_fg < 0.08 {
            continue;
        }

        let mut distinct: bool = true;
        for &existing in &palette {
            let manh_d = (sample.r as i32 - existing.r as i32).abs()
                       + (sample.g as i32 - existing.g as i32).abs()
                       + (sample.b as i32 - existing.b as i32).abs();
            if manh_d < 50 {
                distinct = false;
                break;
            }
        }

        if distinct {
            let c = sample;
            palette.push(*c);
            if palette.len() >= PALETTE_COUNT {
                break;
            }
        }
    }
    palette.sort_unstable_by(|a, b| b.chroma.cmp(&a.chroma));
    return Colorscheme { palette: palette, 
                         background: darkest, 
                         foreground: lightest }
}

pub fn kmeans_generate_colorscheme(img: &DynamicImage) -> Colorscheme {
    const DIVISOR:       usize = 32;
    const SAMPLE_COUNT:  usize = 1024;
    const PALETTE_COUNT: usize = 16;
    
    let w = img.width() as usize;
    let h = img.height() as usize;
    let mut samples: Vec<Color> = Vec::with_capacity(SAMPLE_COUNT);
    
    let step_x = (w / DIVISOR).max(1);
    let step_y = (h / DIVISOR).max(1);
    let mut darkest  = Color {r: 255, g: 255, b: 255, chroma: 0, luminance: 1.0};
    let mut lightest = Color {r: 0, g: 0, b: 0, chroma: 0, luminance: 0.0};
    
    'pixels: for y in (0..h).step_by(step_y) {
        for x in (0..w).step_by(step_x) {
            if samples.len() >= SAMPLE_COUNT {
                break 'pixels;
            }
            let pixel = img.get_pixel(x as u32, y as u32);
            let c = Color::from_rgba(pixel);
            
            if c.luminance < darkest.luminance && c.luminance > 0.05  { darkest = c };
            if c.luminance > lightest.luminance && c.luminance < 0.95 { lightest = c };

            samples.push(c);
        }
    }
    samples.sort_unstable_by(|a, b| b.chroma.cmp(&a.chroma));
    
    let mut centers: Vec<Color> = (0..PALETTE_COUNT)
            .map(|i| samples[i * (SAMPLE_COUNT / PALETTE_COUNT)])
            .collect();
    for _iter in 0..10 {
        let mut r_sum  = [0i32; PALETTE_COUNT];
        let mut g_sum  = [0i32; PALETTE_COUNT];
        let mut b_sum  = [0i32; PALETTE_COUNT];
        let mut counts = [0usize; PALETTE_COUNT];

        for sample in &samples {
            let diff_bg = (sample.luminance - darkest.luminance).abs(); 
            let diff_fg = (sample.luminance - lightest.luminance).abs(); 
            if diff_bg < 0.08 || diff_fg < 0.08 {
                continue;
            }

            let mut best_idx = 0;
            let mut best_dist = f32::INFINITY;

            for (idx, center) in centers.iter().enumerate() {
                let dist = sample.distance_to(center);
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = idx;
                }
            }

            r_sum[best_idx] += sample.r as i32;
            g_sum[best_idx] += sample.g as i32;
            b_sum[best_idx] += sample.b as i32;
            counts[best_idx] += 1;
        }

        for k in 0..PALETTE_COUNT {
            if counts[k] > 0 {
                centers[k].r = (r_sum[k] / counts[k] as i32) as u8;
                centers[k].g = (g_sum[k] / counts[k] as i32) as u8;
                centers[k].b = (b_sum[k] / counts[k] as i32) as u8;

                let max = centers[k].r.max(centers[k].g).max(centers[k].b);
                let min = centers[k].r.min(centers[k].g).min(centers[k].b);
                centers[k].chroma = max - min;
                centers[k].luminance = (0.2126 * centers[k].r as f32 
                                      + 0.7152 * centers[k].g as f32 
                                      + 0.0722 * centers[k].b as f32) / 255.0;
            }
        }
    }
    centers.sort_unstable_by(|a, b| b.chroma.cmp(&a.chroma));
    return Colorscheme { palette: centers, 
                         background: darkest, 
                         foreground: lightest }
}

pub fn ansi_generate_colorscheme(img: &DynamicImage) -> Colorscheme {
    const DIVISOR:       usize = 32;
    const SAMPLE_COUNT:  usize = 1024;

    const ANSI_BASE: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00), (0xcd, 0x00, 0x00), (0x00, 0xcd, 0x00), (0xcd, 0xcd, 0x00),
        (0x00, 0x00, 0xee), (0xcd, 0x00, 0xcd), (0x00, 0xcd, 0xcd), (0xe5, 0xe5, 0xe5),
        (0x7f, 0x7f, 0x7f), (0xff, 0x00, 0x00), (0x00, 0xff, 0x00), (0xff, 0xff, 0x00),
        (0x5c, 0x5c, 0xff), (0xff, 0x00, 0xff), (0x00, 0xff, 0xff), (0xff, 0xff, 0xff),
    ];

    let w = img.width() as usize;
    let h = img.height() as usize;
    let mut samples: Vec<Color> = Vec::with_capacity(SAMPLE_COUNT);
    
    let step_x = (w / DIVISOR).max(1);
    let step_y = (h / DIVISOR).max(1);
    let mut darkest  = Color {r: 255, g: 255, b: 255, chroma: 0, luminance: 1.0};
    let mut lightest = Color {r: 0, g: 0, b: 0, chroma: 0, luminance: 0.0};
    
    'pixels: for y in (0..h).step_by(step_y) {
        for x in (0..w).step_by(step_x) {
            if samples.len() >= SAMPLE_COUNT {
                break 'pixels;
            }
           
            let Some(c) = sample_4by4_area(img, x, y, w, h) else {
                continue;
            };
            if c.luminance < darkest.luminance && c.luminance > 0.05  { darkest = c };
            if c.luminance > lightest.luminance && c.luminance < 0.95 { lightest = c };

            samples.push(c);
        }
    }

    let mut palette = Vec::with_capacity(16);
    
    for &(base_r, base_g, base_b) in &ANSI_BASE {
        let base = Color::from_rgba(Rgba([base_r, base_g, base_b, 255]));
        let mut best_sample = &samples[0];
        let mut best_dist = f32::MAX; 

        for sample in &samples {
            let dist = sample.distance_to(&base);
            if dist < best_dist {
                best_dist = dist;
                best_sample = sample;
            }
        }
        palette.push(*best_sample);
    }

    palette[0] = darkest;
    palette[15] = lightest;

    return Colorscheme { palette, 
                         background: darkest, 
                         foreground: lightest }
}
