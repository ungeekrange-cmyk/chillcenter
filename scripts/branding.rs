//! Development-only, dependency-free SVG palette/motif generator.
//! Compile with the project's pinned rustc; does not run in the app or server.
use std::{env, fs, io, path::Path};

const MASTER: &str = include_str!("../icons/branding/vector/master.svg");

#[derive(Clone, Copy)]
struct Theme {
    key: &'static str,
    ink: &'static str,
    disc: &'static str,
    motif: &'static str,
    detail: &'static str,
}

const THEMES: [Theme; 13] = [
    Theme {
        key: "default",
        ink: "#692535",
        disc: "#EAB76B",
        motif: "",
        detail: "",
    },
    Theme {
        key: "spring-light",
        ink: "#E7EDD9",
        disc: "#426246",
        motif: "",
        detail: "",
    },
    Theme {
        key: "spring-dark",
        ink: "#233F32",
        disc: "#CCDCB4",
        motif: "",
        detail: "",
    },
    Theme {
        key: "summer-light",
        ink: "#FFF3C8",
        disc: "#DA7515",
        motif: "",
        detail: "",
    },
    Theme {
        key: "summer-dark",
        ink: "#8F3E1B",
        disc: "#FFD36E",
        motif: "",
        detail: "",
    },
    Theme {
        key: "autumn-light",
        ink: "#F3DFC5",
        disc: "#8D4930",
        motif: "",
        detail: "",
    },
    Theme {
        key: "autumn-dark",
        ink: "#4E2528",
        disc: "#DFAB65",
        motif: "",
        detail: "",
    },
    Theme {
        key: "winter-light",
        ink: "#E5EEF3",
        disc: "#345771",
        motif: "",
        detail: "",
    },
    Theme {
        key: "winter-dark",
        ink: "#203548",
        disc: "#C4DCE8",
        motif: "",
        detail: "",
    },
    Theme {
        key: "christmas-light",
        ink: "#F5E8CE",
        disc: "#85313B",
        motif: "tree",
        detail: "#8EA78A",
    },
    Theme {
        key: "christmas-dark",
        ink: "#193D2D",
        disc: "#F0D6A0",
        motif: "tree",
        detail: "#8EA78A",
    },
    Theme {
        key: "halloween-light",
        ink: "#FFF0D9",
        disc: "#9C421E",
        motif: "pumpkin",
        detail: "#EF821F",
    },
    Theme {
        key: "halloween-dark",
        ink: "#30202E",
        disc: "#EDAE66",
        motif: "pumpkin",
        detail: "#FF922B",
    },
];

fn render(theme: Theme) -> String {
    let background = if theme.motif == "tree" {
        format!(
            "<use id=\"event-tree\" xlink:href=\"#christmas-tree\" fill=\"{}\"/>",
            theme.detail
        )
    } else {
        String::new()
    };
    let foreground = if theme.motif == "pumpkin" {
        let outline = if theme.key == "halloween-light" {
            theme.disc
        } else {
            theme.ink
        };
        let stem = if theme.key == "halloween-dark" {
            "#FFE4A7"
        } else {
            outline
        };
        let body_outline = if theme.key == "halloween-dark" {
            " stroke=\"#FFE4A7\" stroke-width=\"3\""
        } else {
            ""
        };
        format!(
            "<g id=\"event-pumpkin\"><use xlink:href=\"#pumpkin-body\" fill=\"{}\"{}/><use xlink:href=\"#pumpkin-stem\" fill=\"{}\"/><use xlink:href=\"#pumpkin-ribs\" fill=\"none\" stroke=\"{}\" stroke-width=\"7\" stroke-linecap=\"round\"/></g>",
            theme.detail, body_outline, stem, outline
        )
    } else {
        String::new()
    };
    MASTER
        .replace("#692535", theme.ink)
        .replace("#EAB76B", theme.disc)
        .replace(
            "Chill Center — gabarit commun",
            &format!("Chill Center — {}", theme.key),
        )
        .replace("<!--EVENT_BACKGROUND-->", &background)
        .replace("<!--EVENT_FOREGROUND-->", &foreground)
}

fn main() -> io::Result<()> {
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "icons/branding/vector".to_string());
    let directory = Path::new(&output);
    fs::create_dir_all(directory)?;
    for theme in THEMES {
        fs::write(directory.join(format!("{}.svg", theme.key)), render(theme))?;
    }
    println!("Generated13 self-contained SVGs from one master.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_pumpkin_stem_and_body_do_not_blend_into_sofa_or_disc() {
        let theme = THEMES.iter().find(|t| t.key == "halloween-dark").unwrap();
        assert_ne!(theme.detail, theme.disc);
        let svg = render(*theme);
        assert!(svg.contains("#pumpkin-stem\" fill=\"#FFE4A7\""));
        assert!(svg.contains("#pumpkin-body\" fill=\"#FF922B\" stroke=\"#FFE4A7\""));
    }

    fn definitions(svg: &str) -> &str {
        svg.split_once("<defs>")
            .unwrap()
            .1
            .split_once("</defs>")
            .unwrap()
            .0
    }

    #[test]
    fn every_theme_has_identical_geometry() {
        for theme in THEMES {
            let svg = render(theme);
            assert_eq!(definitions(&svg), definitions(MASTER));
            assert!(svg.contains("cx=\"624\" cy=\"652\" r=\"448\""));
            assert_eq!(svg.matches("xlink:href=\"#brand-foreground\"").count(), 1);
        }
    }

    #[test]
    fn palette_and_motif_placement_are_explicit() {
        for theme in THEMES {
            let svg = render(theme);
            assert!(svg.contains(&format!("fill=\"{}\"", theme.ink)));
            assert!(svg.contains(&format!("fill=\"{}\"", theme.disc)));
            let sofa = svg.find("xlink:href=\"#brand-foreground\"").unwrap();
            assert_eq!(svg.contains("id=\"event-tree\""), theme.motif == "tree");
            assert_eq!(
                svg.contains("id=\"event-pumpkin\""),
                theme.motif == "pumpkin"
            );
            if theme.motif == "tree" {
                assert!(svg.find("id=\"event-tree\"").unwrap() < sofa);
            }
            if theme.motif == "pumpkin" {
                assert!(svg.find("id=\"event-pumpkin\"").unwrap() > sofa);
            }
        }
    }

    #[test]
    fn assets_are_static_self_contained_and_font_independent() {
        for theme in THEMES {
            let svg = render(theme);
            for forbidden in [
                "<script",
                "<foreignObject",
                "<image",
                "<text",
                "<animate",
                "onload=",
                "@import",
            ] {
                assert!(!svg.contains(forbidden), "{forbidden}");
            }
            assert!(!svg.contains("<!--EVENT_"));
            assert_eq!(svg, render(theme));
        }
    }
}
