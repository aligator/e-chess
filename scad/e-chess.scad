renderTopBoard = true;
renderTopGrid = true;
renderGrid = true;
renderBottom = true;
renderElectronicCase = true;
renderElectronicCaseCover = true;
flipElectronicCaseCover = false;

renderPrintable = false;

cutParts = false;

// Also must match the led-strip led distance
fieldSize = 33;

// Board field count. For a normal chess -> 8.
size = 8;

top = 2.0;

topBoardHeight = 0.4;

// Border on each field.
// Note that two borders side by side lead to an effective *2 border.
fieldBorder = 1;

wireRadius = 1.5;

boxHeight = 30;

ledWidth = 11;
ledHeight = 3;
bottomHeight = 5;

// Additional bottomWallSize to the size of the grid.
bottomWallSize = 3;

electronicCaseWidth = 50;
electronicCaseCover = 1;
electronicCaseCoverMagnetDiameter = 10;
electronicCaseCoverMagnetHolderThickness = 3;
electronicCaseCoverMagnetThickness = 3;
electronicCaseCoverStamp = 3;

tolerance = 0.3;

ledWallCutout = 2.0;

coverWidth = electronicCaseWidth - bottomWallSize - 2 * tolerance;
// Just a constant to make cutouts larger for better preview rendering.
c0 = 0.001 + 0;

$fa = 12;
$fs = 1;

gridInner = size * fieldSize;
gridOuter = gridInner + 4 * fieldBorder + 2 * tolerance;
cutPartsSize = cutParts ? 10 : 0;

fullOuterBoard = gridOuter + 2 * bottomWallSize + 2 * tolerance;

module eachGrid() {
  for (i = [0:1:size - 1]) {
    for (j = [0:1:size - 1]) {
      translate([ i * fieldSize, j * fieldSize, 0 ]) children();
    }
  }
}

module cut4(partSize, gap) {
  if (gap == 0) {
    // Fast path to avoid unneeded rendering
    children();
  } else {
    for (i = [0:1:1]) {
      for (j = [0:1:1]) {
        translate(
            [ i * partSize[0] / 2 + i * gap, j * partSize[1] / 2 + j * gap, 0 ])
            intersection() {
          translate([ -i * partSize[0] / 2, -j * partSize[1] / 2 ]) children();
          cube([ partSize[0] / 2, partSize[1] / 2, partSize[2] ]);
        }
      }
    }
  }
}

module cut2(partSize, gap, translation = [ 0, 0, 0 ]) {
  if (gap == 0) {
    // Fast path to avoid unneeded rendering
    children();
  } else {
    i = 0;
    for (j = [0:1:1]) {
      translate([ 0, j * partSize[1] / 2 + j * gap, 0 ]) intersection() {
        translate([ 0, -j * partSize[1] / 2 ]) children();
        translate(translation)
            cube([ partSize[0], partSize[1] / 2, partSize[2] ]);
      }
    }
  }
}

module Field() {
  difference() {
    cube([ fieldSize + fieldBorder * 2, fieldSize + fieldBorder * 2, top ]);
    translate([ fieldBorder, fieldBorder, -c0 ])
        cube([ fieldSize, fieldSize, top + c0 * 2 ]);
  };
}

module TopBoard() {
  translate([ tolerance + fieldBorder, tolerance + fieldBorder, 0 ]) cube([
    gridInner - tolerance * 2, gridInner - tolerance * 2,
    topBoardHeight
  ]);
}

module TopGrid() {
  translate([ fieldBorder + tolerance, fieldBorder + tolerance, 0 ]) {
    // Render border
    translate([
      -tolerance - fieldBorder, -tolerance - fieldBorder, bottomHeight - top -
      topBoardHeight
    ]) difference() {
      cube([
        gridInner + fieldBorder * 4 + tolerance * 2,
        gridInner + fieldBorder * 4 + tolerance * 2,
        boxHeight + topBoardHeight +
        top
      ]);

      translate(
          [ tolerance + fieldBorder * 2, tolerance + fieldBorder * 2,
            -c0 ])
          cube([
            gridInner,  // Use c0 here to fuse the grid with the border.
                               // Otherwise these are handled as two parts.
            gridInner, boxHeight + topBoardHeight + c0 * 2 +
            top
          ]);
    }

    translate(
        [ 0, 0, boxHeight + bottomHeight - top ])
        eachGrid() {
      Field();
    }
  }
}

module Grid() {
  // Render the grid, but remove the outer border as that is rendered separately
  // with the top grid.
  intersection() {
    // Base cube, cuts away the outer border
    translate([
      fieldBorder * 2 + tolerance * 2 + c0,
      fieldBorder * 2 + tolerance * 2 + c0, 0
    ])
        // Note we make the grid even smaller (by tolerance) to make sure it
        // fits nicely in the top-grid.
        cube([
          gridInner - 2 * tolerance - c0 * 2,
          gridInner - 2 * tolerance - c0 * 2,
          boxHeight
        ]);

    // Grid itself
    translate([ fieldBorder + tolerance, fieldBorder + tolerance, 0 ])
        eachGrid(){difference(){
            // Base cube for each field
            cube([
              fieldSize + 2 * fieldBorder, fieldSize + 2 * fieldBorder,
              boxHeight - topBoardHeight - top
            ]);
    // Cut out inner block so that only the outlines are left.
    translate([ fieldBorder, fieldBorder, -c0 ])
        cube([ fieldSize, fieldSize, boxHeight- topBoardHeight + c0 * 2 ]);

    // Wires
    translate([
      wireRadius + fieldBorder * 2 + tolerance,
      fieldSize + fieldBorder * 2 + c0,
      wireRadius + fieldBorder * 2,
    ]) rotate([ 90, 0, 0 ])
        cylinder(h = fieldSize + fieldBorder * 2 + c0 * 2, r = wireRadius);
    translate([
      -c0,
      wireRadius + fieldBorder * 2 + tolerance,
      wireRadius + fieldBorder * 2 + wireRadius * 2,
    ]) rotate([ 90, 0, 90 ])
        cylinder(h = fieldSize + fieldBorder * 2 + c0 * 2, r = wireRadius);
  }
};
}
}

module BottomElectronic() {
  for (i = [0:1:size - 1]) {
    ledLength = i == size - 1 ? gridOuter + bottomWallSize * 2 + c0
                              : gridOuter + tolerance * 3 + ledWallCutout * 2;

    // Led strips
    translate([
      bottomWallSize - ledWallCutout,
      i * fieldSize + bottomWallSize + tolerance + fieldSize / 2 - ledWidth / 2,
      bottomHeight -       ledHeight
    ]) cube([ ledLength, ledWidth, ledHeight  + c0 ]);

    // Wires for the strips.
    translate(
        [ bottomWallSize - ledWallCutout, bottomWallSize, bottomWallSize ])
        cube([ ledWallCutout + c0, gridOuter + 2 * tolerance, boxHeight / 2 ]);

    translate([
      gridOuter + bottomWallSize + tolerance + tolerance - c0, bottomWallSize,
      bottomWallSize
    ])
        cube([
          ledWallCutout + tolerance + c0, gridOuter + 2 * tolerance,
          boxHeight / 2
        ]);
  }

  // Add hole for the wires of the reeds
  translate([
    gridOuter - fieldSize + tolerance,
    bottomWallSize + tolerance + fieldSize / 2 + ledWidth / 2 + fieldBorder +
        fieldSize * (size - 1),
    bottomHeight - 
    ledHeight
  ])
      cube([
        fieldSize + bottomWallSize * 2 + tolerance + c0, ledHeight, ledHeight +
        c0
      ]);
}

module Bottom() {
  difference() {
    cube([ fullOuterBoard, fullOuterBoard, bottomHeight + boxHeight ]);
    translate(
        [ bottomWallSize, bottomWallSize, bottomHeight ])
        cube([
          gridOuter + 2 * tolerance, gridOuter + 2 * tolerance, boxHeight +
          c0
        ]);

    BottomElectronic();
  }
}

module ElectronicCase() {
  difference() {
    cube(
        [ electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight ]);
    translate([ 0, bottomWallSize, bottomWallSize ]) cube([
      c0 + electronicCaseWidth - 1 * bottomWallSize, gridOuter + 2 * tolerance,
      bottomHeight +
      boxHeight
    ]);
  }
}

module ElectronicCaseCover() {
  translate([
    tolerance, bottomWallSize + tolerance, bottomHeight + boxHeight -
    electronicCaseCover
  ]) {
    cube([ coverWidth, gridOuter, electronicCaseCover ]);

    // Magnets
    translate([
      -electronicCaseCoverMagnetDiameter / 2 + (coverWidth) / 2,
      electronicCaseCoverMagnetThickness,
      -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize)
    ])
        cube([
          electronicCaseCoverMagnetDiameter,
          electronicCaseCoverMagnetHolderThickness,
          bottomHeight + boxHeight - electronicCaseCover -
          bottomWallSize
        ]);
    translate([
      -electronicCaseCoverMagnetDiameter / 2 + (coverWidth) / 2,
      gridOuter - electronicCaseCoverMagnetHolderThickness -
          electronicCaseCoverMagnetThickness,
      -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize)
    ])
        cube([
          electronicCaseCoverMagnetDiameter,
          electronicCaseCoverMagnetHolderThickness,
          bottomHeight + boxHeight - electronicCaseCover -
          bottomWallSize
        ]);

    // Stamps at the center
    translate([
      0, gridOuter / 2 - electronicCaseCoverStamp / 2,
      -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize)
    ])
        cube([
          electronicCaseCoverStamp, electronicCaseCoverStamp,
          bottomHeight + boxHeight - electronicCaseCover -
          bottomWallSize
        ]);
    translate([
      coverWidth - electronicCaseCoverStamp,
      gridOuter / 2 - electronicCaseCoverStamp / 2,
      -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize)
    ])
        cube([
          electronicCaseCoverStamp, electronicCaseCoverStamp,
          bottomHeight + boxHeight - electronicCaseCover -
          bottomWallSize
        ]);
  }
}

if (!renderPrintable) {
  if (renderTopBoard) {
    translate([
      fieldBorder + tolerance + bottomWallSize + tolerance,
      fieldBorder + tolerance + bottomWallSize + tolerance,
      boxHeight + bottomHeight - top - topBoardHeight
    ])
        cut4(
            [
              fieldSize * size + 2 * fieldBorder,
              fieldSize * size + 2 * fieldBorder, bottomHeight +
              boxHeight
            ],
            cutPartsSize) TopBoard();
  }

  if (renderTopGrid) {
    translate([ bottomWallSize + tolerance, bottomWallSize + tolerance, 0 ])
        cut4(
            [
              fieldSize * size + 4 * fieldBorder + tolerance * 2,
              fieldSize * size + 4 * fieldBorder + tolerance * 2, bottomHeight +
              boxHeight
            ],
            cutPartsSize) TopGrid();
  }

  if (renderGrid) {
    // Grid, including the wiring for the reed contacts.
    // Open at the bottom, to allow easy wiring.
    translate([
      bottomWallSize + tolerance, bottomWallSize + tolerance, bottomHeight
    ]) cut4([ gridOuter, gridOuter, bottomHeight + boxHeight ], cutPartsSize)
        Grid();
  }

  if (renderBottom) {
    // The bottom embeds the led strip.
    cut4(
        [
          gridOuter + bottomWallSize * 2 + tolerance * 2,
          gridOuter + bottomWallSize * 2 + tolerance * 2, bottomHeight +
          boxHeight
        ],
        cutPartsSize) Bottom();
  }

  translate([ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCase) {
    translate([ fullOuterBoard - c0, 0, 0 ]) cut2(
        [ electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight ],
        cutPartsSize) ElectronicCase();
  };

  translate(
      [ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCaseCover) {
    if (flipElectronicCaseCover) {
      translate([
        tolerance + gridOuter + 2 * bottomWallSize + electronicCaseWidth * 2, 0,
        bottomHeight +
        boxHeight
      ]) rotate([ 0, 180, 0 ])
          cut2(
              [
                coverWidth, gridOuter, electronicCaseCover + bottomHeight +
                boxHeight
              ],
              cutPartsSize, [ 0, bottomWallSize, 0 ]) ElectronicCaseCover();
    } else {
      translate([ fullOuterBoard, 0, 0 ]) cut2(
          [
            coverWidth, gridOuter, electronicCaseCover + bottomHeight +
            boxHeight
          ],
          cutPartsSize, [ 0, bottomWallSize, 0 ]) ElectronicCaseCover();
    }
  }
} else {
  if (renderTopBoard) {
    translate([
      fieldBorder + tolerance + bottomWallSize + tolerance,
      fieldBorder + tolerance + bottomWallSize + tolerance + 20 + gridOuter,
      boxHeight + bottomHeight - boxHeight -
      bottomHeight
    ])
        cut4(
            [
              fieldSize * size + 2 * fieldBorder,
              fieldSize * size + 2 * fieldBorder, bottomHeight +
              boxHeight
            ],
            cutPartsSize) TopBoard();
  }

  if (renderTopGrid) {
    topGridSize = fieldSize * size + 4 * fieldBorder + tolerance * 2;
    translate([
      fullOuterBoard + 40, topGridSize + cutPartsSize + 40 + fullOuterBoard,
      boxHeight +
      bottomHeight
    ]) rotate([ 180, 0, 0 ])
        cut4([ topGridSize, topGridSize, bottomHeight + boxHeight ],
             cutPartsSize) TopGrid();
  }

  if (renderGrid) {
    // Grid, including the wiring for the reed contacts.
    // Open at the bottom, to allow easy wiring.
    translate([
      bottomWallSize + tolerance,
      bottomWallSize + tolerance + 40 + gridOuter * 2,
      bottomHeight - 
      bottomHeight
    ]) cut4([ gridOuter, gridOuter, bottomHeight + boxHeight ], cutPartsSize)
        Grid();
  }

  if (renderBottom) {
    // The bottom embeds the led strip.
    cut4(
        [
          gridOuter + bottomWallSize * 2 + tolerance * 2,
          gridOuter + bottomWallSize * 2 + tolerance * 2, bottomHeight +
          boxHeight
        ],
        cutPartsSize) Bottom();
  }

  translate([ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCase) {
    translate([ fullOuterBoard - c0, 0, 0 ]) cut2(
        [ electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight ],
        cutPartsSize) ElectronicCase();
  };

  translate(
      [ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCaseCover) {
    translate([
      tolerance + gridOuter + 2 * bottomWallSize + electronicCaseWidth * 2, 0,
      bottomHeight +
      boxHeight
    ]) rotate([ 0, 180, 0 ])
        cut2(
            [
              coverWidth, gridOuter, electronicCaseCover + bottomHeight +
              boxHeight
            ],
            cutPartsSize, [ 0, bottomWallSize, 0 ]) ElectronicCaseCover();
  }
}