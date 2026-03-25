


// params
// closure_coords
// prefix (low priority)

// outer |
//       V  inner -> | mod | trait | impl | struct | impl trait | fn             | static
// ------------------------------------------------------------------------      ----------------
// mod               |     |       |      |        |            | .              |       |
// trait             |     |       |      |        |            |                |       |
// impl struct       |     |       |      |        |            |                |       | 
// impl trait        |     |       |      |        |            |                |       |
// fn                |     |       |      |        |            | params         |       |
//                   |     |       |      |        |            | closure_coords |       |
// static            |     |       |      |        |            |                |       |


mod call_params;
mod closure_coords;
