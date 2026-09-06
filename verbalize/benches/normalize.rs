//! Target: a 5,000-character paragraph in well under a millisecond.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use verbalize::{Language, Normalizer};

const SENTENCES: &[&str] = &[
    "Am 1. November 2026 kostet die Fahrkarte 8,80 € und die Fahrt dauert 5–10 Minuten.",
    "Der Blutdruck liegt bei 120/80 mmHg, die Herzfrequenz bei 100-110/min, SpO2 92 %.",
    "Hydrocortison 20-5-0 mg/d, Metformin 1000 mg 2×/Tag, Apixaban 5 mg 2x täglich.",
    "Tel. 030 12 34 56 78, erreichbar von 7:00 bis 9:00 Uhr und 14:00–16:00.",
    "Kalium 5,8 mmol/l, Glukose 210 mg/dl, GFR 42 ml/min/1,73 m², Leukozyten 14 G/l.",
    "Belastungsdyspnoe (NYHA II-III), Herzgeräusch Grad III von VI, Z. n. Myokardinfarkt.",
    "Die Temperatur stieg um 3,5 °C auf 20 °C; der Anteil beträgt 12 % (gerundet).",
    "z. B. 1,5 × 10⁻⁶, p < 0,05, n = 12, H₂O, x², 3/4 Liter, § 25 SGB V, 1990er Jahre.",
    "Im 2. Stock wohnt Fr. Müller seit 3. Oktober; die 3. Klasse trifft sich um 9:00 Uhr.",
    "Die Rechnung über 1.200,50 € ist bis zum 15.03.2024 zu zahlen, zzgl. 19 % MwSt.",
];

fn paragraph(chars: usize) -> String {
    let mut text = String::new();
    for sentence in SENTENCES.iter().cycle() {
        if text.chars().count() >= chars {
            break;
        }
        text.push_str(sentence);
        text.push(' ');
    }
    text
}

const PROSE: &str = "Die Bürgerinitiative traf sich am Abend im Gemeindesaal, um über den geplanten Radweg zu sprechen, \
der die beiden Ortsteile verbinden soll. Nach einer kurzen Begrüßung stellte die Vorsitzende den Stand der \
Planung vor und bat um Wortmeldungen aus dem Publikum. ";

/// Ordinary prose with a number or two per paragraph.
fn typical(chars: usize) -> String {
    let mut text = String::new();
    let mut i = 0;
    while text.chars().count() < chars {
        text.push_str(PROSE);
        text.push_str(SENTENCES[i % SENTENCES.len()]);
        text.push(' ');
        i += 1;
    }
    text
}

fn bench(c: &mut Criterion) {
    let normalizer = Normalizer::new(Language::De);
    let dense = paragraph(5_000);
    let typical = typical(5_000);
    let plain = "Ein Absatz ohne eine einzige Zahl, nur Wörter und Satzzeichen. ".repeat(80);
    let mut group = c.benchmark_group("normalize");
    group.throughput(Throughput::Bytes(typical.len() as u64));
    group.bench_function("de 5000 chars typical", |b| {
        b.iter(|| normalizer.normalize(&typical))
    });
    group.bench_function("de 5000 chars number-dense", |b| {
        b.iter(|| normalizer.normalize(&dense))
    });
    group.bench_function("de 5000 chars no numbers", |b| {
        b.iter(|| normalizer.normalize(&plain))
    });
    group.finish();
    c.bench_function("Normalizer::new(De)", |b| {
        b.iter(|| Normalizer::new(Language::De))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
