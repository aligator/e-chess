renderTopBoard = true;

// Enable multicolor using an extra layer.
renderLayeredMultiColorBoard = true;

// Switches the fields that get the extra layer. (e.g. to change the color on a specific layer manually.)
// Cannot be used together with renderMultiColorBoardColorX.
topBoardLayeredMultiColorEven = false;

// Enables multi-color Color1 (e.g. support for mulit-color printers.)
// Cannot be used together with topBoardLayeredMultiColorEven.
renderMultiColorBoardColor1 = false;
// Enables multi-color Color2 (e.g. support for mulit-color printers.)
// Cannot be used together with topBoardLayeredMultiColorEven.
renderMultiColorBoardColor2 = false;

renderTopGrid = true;
renderGrid = true;
renderBottom = true;
renderElectronicCase = true;
renderElectronicCaseCover = true;
renderReedPins = true;
// Just for debugging
renderMetalPlate = false;
flipElectronicCaseCover = false;

// Experimental - not really nice...
extraReedPinCutout = false;

// More easy to add the wires,
// but may lead to light bleeding into neightbour fields.
electronicCutoutInBorder = true;

renderPrintable = true;

cutParts = false;

// Also must match the led-strip led distance
fieldSize = 33;

// Board field count. For a normal chess -> 8.
size = 8;

top = 2.0;

topBoardHeight = 0.4;
topBoardMultiColorHeight = 0.2;

// Border on each field.
// Note that two borders side by side lead to an effective *2 border.
fieldBorder = 1;

wireRadius = 1.5;

boxHeight = 30;

ledWidth = 11;
ledHeight = 4;
bottomHeight = 5;

// Additional bottomWallSize to the size of the grid.
bottomWallSize = 5;

electronicCaseWidth = 50;
electronicCaseCover = 1;
electronicCaseCoverMagnetDiameter = 10;
electronicCaseCoverMagnetHolderThickness = 3;
electronicCaseCoverMagnetThickness = 3;
electronicCaseCoverStamp = 3;
electronicBreakThrough = 9;

usbCutoutDia = 10;

reedPinBorder = 3;
metalPlateThickness = 0.3;
metalPlateRadius = 7.5;
reedThickness = 3.3;
reedWireThickness = 2;
reedOffset = 5;

tolerance = 0.4;

ledWallCutout = 2.0;

coverWidth = electronicCaseWidth - bottomWallSize - 2 * tolerance;

reedPinHeight = boxHeight - topBoardHeight - top - metalPlateThickness;

// Just a constant to make cutouts larger for better preview rendering.
c0 = 0.01 + 0;

$fa = 12;
$fs = 1;

gridInner = size * fieldSize;
gridOuter = gridInner + 2 * fieldBorder;
cutPartsSize = cutParts ? 10 : 0;

fullOuterBoard = gridOuter + 2 * bottomWallSize + 2 * tolerance;

reedPinWidth = ledWidth + reedPinBorder * 2;

module eachGrid(count = size) {
  for (i = [0:1:count - 1]) {
    for (j = [0:1:count - 1]) {
      translate([i * fieldSize, j * fieldSize, 0]) children();
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
          [i * partSize[0] / 2 + i * gap, j * partSize[1] / 2 + j * gap, 0]
        )
          intersection() {
            translate([-i * partSize[0] / 2, -j * partSize[1] / 2]) children();
            cube([partSize[0] / 2, partSize[1] / 2, partSize[2]]);
          }
      }
    }
  }
}

module cut2(partSize, gap, translation = [0, 0, 0]) {
  if (gap == 0) {
    // Fast path to avoid unneeded rendering
    children();
  } else {
    i = 0;
    for (j = [0:1:1]) {
      translate([0, j * partSize[1] / 2 + j * gap, 0]) intersection() {
          translate([0, -j * partSize[1] / 2]) children();
          translate(translation)
            cube([partSize[0], partSize[1] / 2, partSize[2]]);
        }
    }
  }
}

module Field(height) {
  difference() {
    cube([fieldSize, fieldSize, height]);
    translate([fieldBorder, fieldBorder, -c0]) cube(
        [
          fieldSize - fieldBorder * 2,
          fieldSize - fieldBorder * 2,
          height + c0 * 2,
        ]
      );
  }
  ;
}

module LayeredMultiColorBoard() {
  for (i = [0:1:size - 1]) {
    for (j = [0:1:size - 1]) {
      if ( (i + j) % 2 == (topBoardLayeredMultiColorEven ? 0 : 1)) {

        translate(
          [
            i * fieldSize,
            j * fieldSize,
            topBoardHeight,
          ]
        ) {
          cube(
            [
              fieldSize - tolerance * 2 - fieldBorder * 2,
              fieldSize - tolerance * 2 - fieldBorder * 2,
              topBoardMultiColorHeight,
            ]
          );
        }
        ;
      }
    }
  }
}

module MultiColorBoard1(count) {
  difference() {
    cube(
      [
        fieldSize * count,
        fieldSize * count,
        topBoardHeight,
      ]
    );

    for (i = [0:1:count - 1]) {
      for (j = [0:1:count - 1]) {
        if ( (i + j) % 2 == (renderPrintable ? 0 : 1)) {

          translate(
            [
              i * fieldSize,
              j * fieldSize,
              topBoardHeight - topBoardMultiColorHeight,
            ]
          ) {
            cube(
              [
                fieldSize,
                fieldSize,
                topBoardMultiColorHeight + c0,
              ]
            );
          }
          ;
        }
      }
    }
  }
}

module MultiColorBoard2(count) {
  for (i = [0:1:count - 1]) {
    for (j = [0:1:count - 1]) {
      if ( (i + j) % 2 == (renderPrintable ? 0 : 1)) {

        translate(
          [
            i * fieldSize,
            j * fieldSize,
            topBoardHeight - topBoardMultiColorHeight,
          ]
        ) {
          cube(
            [
              fieldSize,
              fieldSize,
              topBoardMultiColorHeight + c0,
            ]
          );
        }
        ;
      }
    }
  }
}

module BaseBoard() {
  cube(
    [
      gridInner - tolerance * 2 - fieldBorder * 2,
      gridInner - tolerance * 2 - fieldBorder * 2,
      topBoardHeight,
    ]
  );
}

module TopBoard() {
  // When rendering printable and with the multi color approach - render only one of the 4 parts.
  // Otherwise the handling in the slicer is hard.
  // You have to print it 4 times.
  multiColorSize = renderPrintable ? size / 2 : size;

  difference() {
    union() {
      if (!renderMultiColorBoardColor1 && !renderMultiColorBoardColor2) {
        translate([fieldBorder, fieldBorder, 0]) {
          BaseBoard();
          if (renderLayeredMultiColorBoard) {
            LayeredMultiColorBoard();
          }
        }
      }
      if (renderMultiColorBoardColor1) {
        MultiColorBoard1(multiColorSize);
      }
      ; if (renderMultiColorBoardColor2) {
        MultiColorBoard2(multiColorSize);
      }
      ;
    }
    ;

    // Cut away borders around the 4 slots.
    translate([0, (gridInner / 2) - tolerance * 2 - fieldBorder, -c0])
      cube([gridInner, fieldBorder * 2 + tolerance * 2, topBoardHeight + c0 * 2]);
    translate([(gridInner / 2) - tolerance * 2 - fieldBorder, 0, -c0])
      cube([fieldBorder * 2 + tolerance * 2, gridInner, topBoardHeight + c0 * 2]);

    translate([0, 0, -c0])
      difference() {
        cube(
          [
            gridInner,
            gridInner,
            topBoardHeight + topBoardMultiColorHeight + 2 * c0,
          ]
        );
        translate([fieldBorder, fieldBorder, 0]) {
          cube(
            [
              gridInner - tolerance * 2 - fieldBorder * 2,
              gridInner - tolerance * 2 - fieldBorder * 2,
              topBoardHeight + topBoardMultiColorHeight + 2 * c0,
            ]
          );
        }
        ;
      }
    ;
  }
}

module TopGrid() {
  difference() {
    union() {
      // Render border
      translate([0, 0, bottomHeight]) difference() {
          cube(
            [
              gridInner + fieldBorder * 2,
              gridInner + fieldBorder * 2,
              boxHeight,
            ]
          );

          translate(
            [
              fieldBorder * 2,
              fieldBorder * 2,
              -c0,
            ]
          )
            cube(
              [
                gridInner - fieldBorder * 2, // Use c0 here to fuse the grid with the border.
                // Otherwise these are handled as two parts.
                gridInner - fieldBorder * 2,
                boxHeight + topBoardHeight + c0 * 2 + top,
              ]
            );
        }

      translate([fieldBorder, fieldBorder, boxHeight + bottomHeight - top])
        eachGrid() {
          Field(top);
        }

      // Add middle borders around the 4 slots.
      // But not down to the bottom, as that is done by the bottom part to improve glueability.
      // Let a small tolerance between.
      translate([0, (gridInner / 2), bottomHeight + boxHeight / 2 + tolerance])
        cube([gridInner, fieldBorder * 2, boxHeight / 2 - tolerance]);
      translate([(gridInner / 2), 0, bottomHeight + boxHeight / 2 + tolerance])
        cube([fieldBorder * 2, gridInner, boxHeight / 2 - tolerance]);
    }
    ;

    // Cut from the outer wall which would collide with the borders that are in the bottom part
    translate([-tolerance, (gridInner / 2) - tolerance, bottomHeight])
      cube([gridOuter + tolerance * 2, fieldBorder * 2 + tolerance * 2, boxHeight / 2 + tolerance]);
    translate([(gridInner / 2) - tolerance, -tolerance, bottomHeight])
      cube([fieldBorder * 2 + tolerance * 2, gridOuter + tolerance * 2, boxHeight / 2 + tolerance]);
  }
}

module Grid() {
  // Note this creates the grid only for the half size.
  // It is then added 4 times.
  // Between the instances is a gap which will be filled by the top grid and the bottom part.

  // This are the four slots of the bottom module.
  translate(
    [
      gridOuter / 2,
      gridOuter / 2,
      0,
    ]
  )for (deg = [0, 90, 180, 270])
    rotate(deg) {

      // Render the grid, but remove the outer border as that is rendered
      // separately with the top grid.

      intersection() {
        // Base cube, cuts away the outer border
        translate(
          [
            fieldBorder * 1 + tolerance,
            fieldBorder * 1 + tolerance,
            0,
          ]
        )
          // Note we make the grid even smaller (by tolerance) to make sure it
          // fits nicely in the top-grid.
          cube(
            [
              (gridInner / 2) - 2 * fieldBorder - 2 * tolerance - c0 * 2,
              (gridInner / 2) - 2 * fieldBorder - 2 * tolerance - c0 * 2,
              boxHeight,
            ]
          );

        // Grid itself
        eachGrid(size / 2) {
          Field(boxHeight - topBoardHeight - top - tolerance);
        }
      }
    }
}

module LedStripWiresInWalls(type) {
  // It is based on the field size if it is even or odd.
  even = size % 2 == 0 ? type : !type;

  moduloNum = even ? 0 : 1;
  startAt = even ? 1 : 0;

  cutoutHeight = boxHeight * 0.9;

  for (i = [startAt:1:size - 1]) {
    if (i % 2 == moduloNum) {
      translate(
        [
          0,
          i * fieldSize + bottomWallSize + tolerance + fieldBorder + fieldSize / 2 - ledWidth - fieldSize,
          bottomWallSize,
        ]
      ) {
        cube(
          [
            ledWallCutout + c0,
            ledWidth + fieldSize + ledWidth,
            cutoutHeight,
          ]
        );
      }
    }
  }
}

module BottomElectronic() {
  for (i = [0:1:size - 1]) {
    ledLength =
      i == size - 1 ? gridOuter + tolerance * 2 + ledWallCutout + electronicCaseWidth
      : gridOuter + tolerance * 3 + ledWallCutout * 2;

    // Led strips
    translate(
      [
        bottomWallSize - ledWallCutout,
        i * fieldSize + bottomWallSize + tolerance + fieldBorder + fieldSize / 2 - ledWidth / 2,
        bottomHeight - ledHeight,
      ]
    ) cube([ledLength, ledWidth, ledHeight + c0]);

    // Wires for the strips at the sides.
    translate([bottomWallSize - ledWallCutout, 0, 0])
      LedStripWiresInWalls(false);

    translate(
      [
        gridOuter + bottomWallSize + 2 * tolerance - c0,
        0,
        0,
      ]
    )
      LedStripWiresInWalls(true);
  }

  cutoutHeight = electronicCutoutInBorder ? boxHeight : wireRadius * 2 + c0;

  // Wires for the reeds. (vertical)
  for (i = [0:1:size - 1]) {
    translate(
      [
        i * fieldSize + bottomWallSize + 2 * tolerance + fieldBorder * 2,
        bottomWallSize + tolerance,
        bottomHeight - wireRadius * 2,
      ]
    )
      cube(
        [
          wireRadius * 2,
          size * fieldSize,
          cutoutHeight,
        ]
      );
  }

  // Wires for the reeds. (horizontal)
  for (i = [0:1:size - 1]) {
    translate(
      [
        bottomWallSize + tolerance + fieldSize / 2,
        i * fieldSize + bottomWallSize + (fieldSize - (fieldBorder * 2 + fieldBorder * 2)), // + bottomWallSize + 2 * tolerance + fieldBorder * 2,
        bottomHeight - wireRadius * 2,
      ]
    )
      cube(
        [
          (size) * fieldSize - fieldSize / 2,
          wireRadius * 2,
          cutoutHeight,
        ]
      );
  }

  // Add hole for the wires of the reeds
  translate(
    [
      gridOuter - fieldSize + tolerance,
      bottomWallSize + tolerance + fieldSize / 2 + ledWidth / 2 + fieldBorder + reedPinBorder + fieldSize * (size - 1),
      bottomHeight - ledHeight,
    ]
  )
    cube(
      [
        fieldSize + bottomWallSize * 2 + tolerance + electronicCaseWidth - bottomWallSize,
        electronicBreakThrough,
        ledHeight + c0,
      ]
    );
}

module Bottom() {
  difference() {
    cube([fullOuterBoard, fullOuterBoard, bottomHeight + boxHeight]);
    translate([bottomWallSize, bottomWallSize, bottomHeight]) {
      // This are the four slots of the bottom module.
      translate(
        [
          gridOuter / 2 + 1 * tolerance,
          gridOuter / 2 + 1 * tolerance,
          0,
        ]
      ) {
        for (deg = [0, 90, 180, 270])
          rotate(deg)
            translate([fieldBorder, fieldBorder, 0]) cube(
                [
                  gridOuter / 2 - fieldBorder + 1 * tolerance,
                  gridOuter / 2 - fieldBorder + 1 * tolerance,
                  boxHeight + c0,
                ]
              );
      }

      // And cut it at the top, so that the inserted grid can take part of it.
      translate([0, 0, boxHeight / 2]) cube(
          [
            gridOuter + 2 * tolerance,
            gridOuter + 2 * tolerance,
            boxHeight / 2 + c0,
          ]
        );
    }

    BottomElectronic();
  }

  if (renderReedPins) {
    translate(
      [
        bottomWallSize + tolerance + fieldBorder,
        bottomWallSize + tolerance + fieldBorder,
        bottomHeight,
      ]
    ) {
      eachGrid() {
        translate(
          [
            fieldSize / 2 - metalPlateRadius,
            fieldSize / 2 - reedPinWidth / 2,
            0,
          ]
        ) ReedPin();
      }
    }
  }
}

module ElectronicCase() {
  difference() {
    cube(
      [electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight]
    );
    translate([0, bottomWallSize, bottomWallSize]) cube(
        [
          c0 + electronicCaseWidth - 1 * bottomWallSize,
          gridOuter + 2 * tolerance,
          bottomHeight + boxHeight,
        ]
      );

    translate([-fullOuterBoard, 0, 0]) BottomElectronic();

    // Hole for cable.
    translate(
      [
        (electronicCaseWidth - bottomWallSize) / 2,
        fullOuterBoard + c0,
        boxHeight / 2,
      ]
    )
      rotate([90, 0, 0])
        cylinder(h=bottomWallSize + 2 * c0, d=usbCutoutDia);
  }
}

module ElectronicCaseCover() {
  translate(
    [
      tolerance,
      bottomWallSize + tolerance,
      bottomHeight + boxHeight - electronicCaseCover,
    ]
  ) {
    cube([coverWidth, gridOuter, electronicCaseCover]);

    // Magnets
    translate(
      [
        -electronicCaseCoverMagnetDiameter / 2 + (coverWidth) / 2,
        electronicCaseCoverMagnetThickness,
        -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize),
      ]
    )
      cube(
        [
          electronicCaseCoverMagnetDiameter,
          electronicCaseCoverMagnetHolderThickness,
          bottomHeight + boxHeight - electronicCaseCover - bottomWallSize,
        ]
      );
    translate(
      [
        -electronicCaseCoverMagnetDiameter / 2 + (coverWidth) / 2,
        gridOuter - electronicCaseCoverMagnetHolderThickness - electronicCaseCoverMagnetThickness,
        -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize),
      ]
    )
      cube(
        [
          electronicCaseCoverMagnetDiameter,
          electronicCaseCoverMagnetHolderThickness,
          bottomHeight + boxHeight - electronicCaseCover - bottomWallSize,
        ]
      );

    // Stamps at the center
    translate(
      [
        0,
        gridOuter / 2 - electronicCaseCoverStamp / 2,
        -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize),
      ]
    )
      cube(
        [
          electronicCaseCoverStamp,
          electronicCaseCoverStamp,
          bottomHeight + boxHeight - electronicCaseCover - bottomWallSize,
        ]
      );
    translate(
      [
        coverWidth - electronicCaseCoverStamp,
        gridOuter / 2 - electronicCaseCoverStamp / 2,
        -(bottomHeight + boxHeight - electronicCaseCover - bottomWallSize),
      ]
    )
      cube(
        [
          electronicCaseCoverStamp,
          electronicCaseCoverStamp,
          bottomHeight + boxHeight - electronicCaseCover - bottomWallSize,
        ]
      );
  }
}

module ReedPin() {

  metalPlateDia = metalPlateRadius * 2;
  cutoutWidthH = metalPlateDia - 2 * reedPinBorder;
  cutoutWidthV = reedPinWidth - 2 * reedPinBorder;
  cutoutHeight = reedPinHeight - reedThickness - reedPinBorder;

  diagonal = sqrt(metalPlateDia ^ 2 + reedPinWidth ^ 2);

  difference() {
    // Reed boxHeight - metalPlateThicknesspin
    cube([metalPlateDia, reedPinWidth, reedPinHeight]);

    translate([0, 0, reedPinHeight - reedThickness])
      rotate(atan(reedPinWidth / metalPlateDia)) {
        // Reed
        translate([(diagonal - metalPlateDia) / 2 + reedOffset, -reedThickness / 2, 0])
          cube([diagonal, reedThickness, reedThickness + c0]);

        // Wire
        translate([(diagonal - metalPlateDia) / 2 + reedOffset - diagonal, -reedWireThickness / 2, 0])
          cube([diagonal, reedWireThickness, reedThickness + c0]);
      }
    // Cutouts
    translate([reedPinBorder, -c0, -c0])
      cube([cutoutWidthH, reedPinWidth + 2 * c0, cutoutHeight + c0]);
    translate([-c0, reedPinBorder, -c0])
      cube([reedPinWidth + 2 * c0, cutoutWidthV, cutoutHeight + c0]);

    if (extraReedPinCutout) {
      translate([reedPinBorder, -c0, -c0])
        cube([cutoutWidthH, reedPinBorder / 2 + c0, reedPinHeight + 2 * c0]);
      translate([reedPinBorder, reedPinWidth - reedPinBorder / 2 + c0, -c0])
        cube([cutoutWidthH, reedPinBorder / 2 + c0, reedPinHeight + 2 * c0]);
    }
  }

  if (renderMetalPlate) {
    translate(
      [
        metalPlateRadius,
        reedPinWidth / 2,
        reedPinHeight,
      ]
    )
      cylinder(h=metalPlateThickness, r=metalPlateRadius);
  }
}

if (!renderPrintable) {
  if (renderTopBoard) {
    translate(
      [
        fieldBorder + 2 * tolerance + bottomWallSize,
        fieldBorder + 2 * tolerance + bottomWallSize,
        boxHeight + bottomHeight - top - topBoardHeight,
      ]
    )
      cut4(
        [
          fieldSize * size - tolerance * 2,
          fieldSize * size - tolerance * 2,
          bottomHeight + boxHeight,
        ],
        cutPartsSize
      ) TopBoard();
  }

  if (renderTopGrid) {
    translate([bottomWallSize + tolerance, bottomWallSize + tolerance, 0])
      cut4(
        [
          fieldSize * size + 2 * fieldBorder,
          fieldSize * size + 2 * fieldBorder,
          bottomHeight + boxHeight,
        ],
        cutPartsSize
      ) TopGrid();
  }

  if (renderGrid) {
    // Grid, including the wiring for the reed contacts.
    // Open at the bottom, to allow easy wiring.
    translate(
      [
        bottomWallSize + tolerance,
        bottomWallSize + tolerance,
        bottomHeight,
      ]
    ) cut4([gridOuter, gridOuter, bottomHeight + boxHeight], cutPartsSize)
        Grid();
  }

  if (renderBottom) {
    // The bottom embeds the led strip.
    cut4(
      [
        gridOuter + bottomWallSize * 2 + tolerance * 2,
        gridOuter + bottomWallSize * 2 + tolerance * 2,
        bottomHeight + boxHeight,
      ],
      cutPartsSize
    ) Bottom();
  }

  translate([cutParts ? cutPartsSize : 0, 0, 0]) if (renderElectronicCase) {
    translate([fullOuterBoard - c0, 0, 0]) cut2(
        [electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight],
        cutPartsSize
      ) ElectronicCase();
  }
  ;

  translate(
    [cutParts ? cutPartsSize : 0, 0, 0]
  ) if (renderElectronicCaseCover) {
    if (flipElectronicCaseCover) {
      translate(
        [
          tolerance + gridOuter + 2 * bottomWallSize + electronicCaseWidth * 2,
          0,
          bottomHeight + boxHeight,
        ]
      ) rotate([0, 180, 0])
          cut2(
            [
              coverWidth,
              gridOuter,
              electronicCaseCover + bottomHeight + boxHeight,
            ],
            cutPartsSize, [0, bottomWallSize, 0]
          ) ElectronicCaseCover();
    } else {
      translate([fullOuterBoard, 0, 0]) cut2(
          [
            coverWidth,
            gridOuter,
            electronicCaseCover + bottomHeight + boxHeight,
          ],
          cutPartsSize, [0, bottomWallSize, 0]
        ) ElectronicCaseCover();
    }
  }
} else {
  if (renderTopBoard) {
    translate(
      [
        fieldBorder + tolerance + bottomWallSize + tolerance,
        fieldBorder + tolerance + bottomWallSize + tolerance + 20 + gridOuter,
        boxHeight + bottomHeight - boxHeight - bottomHeight,
      ]
    )
      cut4(
        [
          fieldSize * size - tolerance * 2,
          fieldSize * size - tolerance * 2,
          bottomHeight + boxHeight,
        ],
        cutPartsSize
      ) TopBoard();
  }

  if (renderTopGrid) {
    topGridSize = fieldSize * size + 4 * fieldBorder + tolerance * 2;
    translate(
      [
        fullOuterBoard + 40,
        topGridSize + cutPartsSize + 40 + fullOuterBoard,
        boxHeight + bottomHeight,
      ]
    ) rotate([180, 0, 0])
        cut4(
          [
            fieldSize * size + 2 * fieldBorder,
            fieldSize * size + 2 * fieldBorder,
            bottomHeight + boxHeight,
          ],
          cutPartsSize
        ) TopGrid();
  }

  if (renderGrid) {
    // Grid, including the wiring for the reed contacts.
    // Open at the bottom, to allow easy wiring.
    translate(
      [
        bottomWallSize + tolerance,
        bottomWallSize + tolerance + 40 + gridOuter * 2,
        bottomHeight - bottomHeight,
      ]
    ) cut4([gridOuter, gridOuter, bottomHeight + boxHeight], cutPartsSize)
        Grid();
  }

  if (renderBottom) {
    // The bottom embeds the led strip.
    cut4(
      [
        gridOuter + bottomWallSize * 2 + tolerance * 2,
        gridOuter + bottomWallSize * 2 + tolerance * 2,
        bottomHeight + boxHeight,
      ],
      cutPartsSize
    ) Bottom();
  }

  translate([cutParts ? cutPartsSize : 0, 0, 0]) if (renderElectronicCase) {
    translate([fullOuterBoard - c0, 0, 0]) cut2(
        [electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight],
        cutPartsSize
      ) ElectronicCase();
  }
  ;

  translate(
    [cutParts ? cutPartsSize : 0, 0, 0]
  ) if (renderElectronicCaseCover) {
    translate(
      [
        tolerance + gridOuter + 2 * bottomWallSize + electronicCaseWidth * 2,
        0,
        bottomHeight + boxHeight,
      ]
    ) rotate([0, 180, 0])
        cut2(
          [
            coverWidth,
            gridOuter,
            electronicCaseCover + bottomHeight + boxHeight,
          ],
          cutPartsSize, [0, bottomWallSize, 0]
        ) ElectronicCaseCover();
  }
}
