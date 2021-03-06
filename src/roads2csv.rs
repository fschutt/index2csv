//! Converts input roads to a final CSV

use std::{fmt, collections::{BTreeMap, BTreeSet}};

/// Name of one street (such as `"Canterbury Road"`)
#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct StreetName(pub String);

impl fmt::Display for StreetName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Input street to the deduplicator - the street must have a 
/// name and a position (such as `"A9"`)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputStreetValue {
    pub street_name: StreetName,
    pub position: GridPosition,
}

/// Grid position such as "A9", "B4" or similar
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridPosition {
    pub column: String,
    pub row: usize,
}

impl fmt::Display for GridPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.column, self.row)
    }
}

/// Deduplicates road names, merging the roads by their name
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeduplicatedRoads {
    pub roads: BTreeMap<StreetName, BTreeSet<GridPosition>>,
}

impl DeduplicatedRoads {
    /// Deduplicates road names, i.e.:
    ///
    /// Input:
    ///
    /// ```no_run,ignore
    /// Mayer Street A4
    /// Mayer Street A5
    /// Mayer Street A6
    /// ```
    ///
    /// Output:
    ///
    /// ```no_run,ignore
    /// Mayer Street -> [A4, A5, A6]
    /// ```
    ///
    /// The output road name positions are ordered.
    pub fn from_streets(streets: &[InputStreetValue]) -> Self {
        let mut deduplicated_names = BTreeMap::new();

        for input_street in streets {
            deduplicated_names
            .entry(input_street.street_name.clone())
            .or_insert_with(|| BTreeSet::new())
            .insert(input_street.position.clone());
        }

        Self { roads: deduplicated_names }
    }

    /// Processes road names (`[A1, A2]` => `A1-A2`) if they span less than 2 grids.
    /// 
    /// Processing road names in a cartographic manner is tricky. For example, a 
    /// street that appears in two locations on the map (such as a city having 
    /// the same street name as a neighbouring city). Because of this, street name
    /// processing can't be fully automated, since there are always weird edge cases 
    /// to worry about. However, 90% of roads aren't like that.
    /// 
    /// Because of this limitation `process()` gives you two types of roads back: 
    /// - `ProcessedRoadName` is for roads that span only 1 or 2 grid cells 
    /// (i.e. `"Canterbury Road" => A9`, `"Canterbury Road" => A9-A10`).
    /// In these cases (which cover 90% of street index names), the mapping is not
    /// ambigouus.
    /// 
    /// `UnprocessedRoadName` is for anything else (e.g. `"Canterbury Road" => [A9, A10, E1, E2]`. 
    /// Usually these roads need to be manually reviewed - it could likely be that 
    /// there are two roads `"Canterbury Road" => A9-10;E1-E2`, but it could also
    /// be that the road is just one road and part of it is just clipped off the map,
    /// in which case you'd write `"Canterbury Road" => A9-E2`. 
    pub fn process(&self) -> (ProcessedRoadNames, UnprocessedRoadNames) {

        let mut processed = BTreeMap::new();
        let mut unprocessed = BTreeMap::new();

        for (road_name, positions) in &self.roads {
            let positions_vec = positions.into_iter().cloned().collect::<Vec<GridPosition>>();
            match positions_vec.len() {
                0 => { },
                1 => { processed.insert(road_name.clone(), FinalizedGridPositon::SingleRect(positions_vec[0].clone())); }
                2 => { processed.insert(road_name.clone(), FinalizedGridPositon::TwoRect(positions_vec[0].clone(), positions_vec[1].clone())); }
                _ => { unprocessed.insert(road_name.clone(), positions_vec); }
            }
        }

        (ProcessedRoadNames {
            processed: processed.into_iter().map(|(k, v)| ProcessedRoad { name: k, position: v }).collect(),
        },
        UnprocessedRoadNames {
            unprocessed: unprocessed.into_iter().map(|(k, v)| UnprocessedRoad { name: k, positions: v }).collect(),
        })
    }
}

#[test]
fn test_deduplicate_streets() {
    let input = [
        InputStreetValue {
            street_name: StreetName(String::from("Valley View Road")),
            position: GridPosition {
                column: String::from("A"),
                row: 4,
            }
        },
        InputStreetValue {
            street_name: StreetName(String::from("Valley View Road")),
            position: GridPosition {
                column: String::from("A"),
                row: 5,
            }
        },
        InputStreetValue {
            street_name: StreetName(String::from("Valley View Road")),
            position: GridPosition {
                column: String::from("B"),
                row: 6,
            }
        },
    ];

    // "Valley View Road" -> ["A4", "A5", "B6"]
    let mut output_expected = BTreeMap::new();
    let mut valley_view_road_expected = BTreeSet::new();
    valley_view_road_expected.insert(GridPosition { column: String::from("A"), row: 4 });
    valley_view_road_expected.insert(GridPosition { column: String::from("A"), row: 5 });
    valley_view_road_expected.insert(GridPosition { column: String::from("B"), row: 6 });
    output_expected.insert(StreetName(String::from("Valley View Road")), valley_view_road_expected);

    assert_eq!(DeduplicatedRoads::from_streets(&input), DeduplicatedRoads { roads: output_expected });
}

#[test]
fn test_format_street() {
    let street_grid_1 = GridPosition { column: String::from("A"), row: 9 };
    let street_grid_2 = GridPosition { column: String::from("I"), row: 5 };

    let road_pos_1 = FinalizedGridPositon::TwoRect(street_grid_1.clone(), street_grid_2);
    assert_eq!(format!("{}", street_grid_1), String::from("A9"));
    assert_eq!(format!("{}", road_pos_1), String::from("A9-I5"));
}

/// Wrapper for grid positions that span less than 2 grid cells
pub enum FinalizedGridPositon {
    /// Road is contained within a single rect, i.e. "Valley Road -> A6"
    SingleRect(GridPosition),
    /// Road crosses exactly two grids
    TwoRect(GridPosition, GridPosition),
}

impl fmt::Display for FinalizedGridPositon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::FinalizedGridPositon::*;
        // single rect: "A9"
        // two rects "A9-B2"
        match self {
            SingleRect(single) => write!(f, "{}", single),
            TwoRect(a, b) => write!(f, "{}-{}", a, b),
        }
    }
}

/// Road name that spans less than 2 grid cells
pub struct ProcessedRoad {
    pub name: StreetName,
    pub position: FinalizedGridPositon,
}

impl fmt::Display for ProcessedRoad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\t{}", self.name, self.position)
    }
}

/// Road name that spans more than 2 grid cells
pub struct UnprocessedRoad {
    pub name: StreetName,
    pub positions: Vec<GridPosition>,
}

impl fmt::Display for UnprocessedRoad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let unprocessed_string = self.positions.iter().map(|pos| format!("{}", pos)).collect::<Vec<String>>().join("\t");
        write!(f, "{}\t{}", self.name, unprocessed_string)
    }
}

/// Simple wrapper for `Vec<ProcessedRoad>` with `.to_csv()` exporting function
pub struct ProcessedRoadNames {
    pub processed: Vec<ProcessedRoad>,
}

impl ProcessedRoadNames {
    pub fn to_csv(&self, delimiter: &str) -> String {
        self.processed.iter().map(|processed_road| 
            format!("{}{}{}", processed_road.name, delimiter, processed_road.position))
        .collect::<Vec<String>>()
        .join("\r\n")
    }
}

/// Simple wrapper for `Vec<UnprocessedRoad>` with `.to_csv()` exporting function
pub struct UnprocessedRoadNames {
    pub unprocessed: Vec<UnprocessedRoad>,
}

impl UnprocessedRoadNames {
    pub fn to_csv(&self, delimiter: &str) -> String {
        self.unprocessed.iter().map(|unprocessed_road| {
            let unprocessed_string = unprocessed_road.positions
                .iter()
                .map(|pos| format!("{}", pos))
                .collect::<Vec<String>>()
                .join(delimiter);
            format!("{}{}{}", unprocessed_road.name, delimiter, unprocessed_string)
        })
        .collect::<Vec<String>>()
        .join("\r\n")
    }
}