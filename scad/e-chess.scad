renderTopBoard = true;
renderTopGrid = true;
renderGrid = true;
renderBottom = true;
renderElectronicCase = true;
renderElectronicCaseCover = true;
renderReedPins = true;
// Just for debugging
renderMetalPlate = false;
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
bottomWallSize = 5;

electronicCaseWidth = 50;
electronicCaseCover = 1;
electronicCaseCoverMagnetDiameter = 10;
electronicCaseCoverMagnetHolderThickness = 3;
electronicCaseCoverMagnetThickness = 3;
electronicCaseCoverStamp = 3;

reedPinBorder = 2;
metalPlateThickness = 0.2;
metalPlateRadius = 5;
reedThickness = 3;
reedWireThickness = 2;
reedOffset = 4;

tolerance = 0.3;

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

module eachGrid()
{
    for (i = [0:1:size - 1]) {
        for (j = [0:1:size - 1]) {
            translate([ i * fieldSize, j * fieldSize, 0 ]) children();
        }
    }
}

module cut4(partSize, gap)
{
    if (gap == 0) {
        // Fast path to avoid unneeded rendering
        children();
    } else {
        for (i = [0:1:1]) {
            for (j = [0:1:1]) {
                translate(
                    [ i * partSize[0] / 2 + i * gap, j * partSize[1] / 2 + j * gap, 0 ])
                    intersection()
                {
                    translate([ -i * partSize[0] / 2, -j * partSize[1] / 2 ]) children();
                    cube([ partSize[0] / 2, partSize[1] / 2, partSize[2] ]);
                }
            }
        }
    }
}

module cut2(partSize, gap, translation = [ 0, 0, 0 ])
{
    if (gap == 0) {
        // Fast path to avoid unneeded rendering
        children();
    } else {
        i = 0;
        for (j = [0:1:1]) {
            translate([ 0, j * partSize[1] / 2 + j * gap, 0 ]) intersection()
            {
                translate([ 0, -j * partSize[1] / 2 ]) children();
                translate(translation)
                    cube([ partSize[0], partSize[1] / 2, partSize[2] ]);
            }
        }
    }
}

module Field(height)
{
    difference()
    {
        cube([ fieldSize, fieldSize, height ]);
        translate([ fieldBorder, fieldBorder, -c0 ]) cube([
            fieldSize - fieldBorder * 2, fieldSize - fieldBorder * 2, height + c0 * 2
        ]);
    };
}

module TopBoard()
{
    translate([ fieldBorder, fieldBorder, 0 ])
        cube([
            gridInner - tolerance * 2 - fieldBorder * 2,
            gridInner - tolerance * 2 - fieldBorder * 2,
            topBoardHeight
        ]);
}

module TopGrid()
{
    // Render border
    translate([ 0, 0, bottomHeight ]) difference()
    {
        cube([
            gridInner + fieldBorder * 2, gridInner + fieldBorder * 2,
            boxHeight
        ]);

        translate([ fieldBorder * 2, fieldBorder * 2,
            -c0 ])
            cube([
                gridInner - fieldBorder * 2, // Use c0 here to fuse the grid with the border.
                                             // Otherwise these are handled as two parts.
                gridInner - fieldBorder * 2, boxHeight + topBoardHeight + c0 * 2 +
                top
            ]);
    }

    translate([ fieldBorder, fieldBorder, boxHeight + bottomHeight - top ])
        eachGrid()
    {
        Field(top);
    }
}

module Grid()
{
    // Render the grid, but remove the outer border as that is rendered
    // separately with the top grid.
    intersection()
    {
        // Base cube, cuts away the outer border
        translate([
            fieldBorder * 2 + tolerance,
            fieldBorder * 2 + tolerance,
            0
        ])
            // Note we make the grid even smaller (by tolerance) to make sure it
            // fits nicely in the top-grid.
            cube([
                gridInner - 2 * fieldBorder - 2 * tolerance - c0 * 2,
                gridInner - 2 * fieldBorder - 2 * tolerance - c0 * 2,
                boxHeight
            ]);

        // Grid itself
        translate([ fieldBorder, fieldBorder, 0 ])
        {
            eachGrid()
            {
                difference()
                {
                    Field(boxHeight - topBoardHeight - top - tolerance);

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
            }
        }
    }
}

module BottomElectronic()
{
    for (i = [0:1:size - 1]) {
        ledLength = i == size - 1 ? gridOuter + bottomWallSize * 2 + c0
                                  : gridOuter + tolerance * 3 + ledWallCutout * 2;

        // Led strips
        translate([
            bottomWallSize - ledWallCutout,
            i * fieldSize + bottomWallSize + tolerance + fieldBorder + fieldSize / 2 - ledWidth / 2,
            bottomHeight -
            ledHeight
        ]) cube([ ledLength, ledWidth, ledHeight + c0 ]);

        // Wires for the strips.
        translate([
            bottomWallSize - ledWallCutout,
            bottomWallSize,
            bottomWallSize
        ])
            cube([ ledWallCutout + c0, gridOuter + 2 * tolerance, boxHeight / 2 ]);

        translate([
            gridOuter + bottomWallSize + 2 * tolerance - c0,
            bottomWallSize,
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
        bottomWallSize + tolerance + fieldSize / 2 + ledWidth / 2 + fieldBorder + wireRadius + fieldSize * (size - 1),
        bottomHeight -
        ledHeight
    ])
        cube([
            fieldSize + bottomWallSize * 3 + tolerance + c0,
            wireRadius * 4,
            ledHeight +
            c0
        ]);
}

module Bottom()
{
    difference()
    {
        cube([ fullOuterBoard, fullOuterBoard, bottomHeight + boxHeight ]);
        translate([ bottomWallSize, bottomWallSize, bottomHeight ])
            cube([
                gridOuter + 2 * tolerance,
                gridOuter + 2 * tolerance,
                boxHeight +
                c0
            ]);

        BottomElectronic();
    }
}

module ElectronicCase()
{
    difference()
    {
        cube(
            [ electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight ]);
        translate([ 0, bottomWallSize, bottomWallSize ]) cube([
            c0 + electronicCaseWidth - 1 * bottomWallSize, gridOuter + 2 * tolerance,
            bottomHeight +
            boxHeight
        ]);

        translate([ -fullOuterBoard, 0, 0 ]) BottomElectronic();
    }
}

module ElectronicCaseCover()
{
    translate([
        tolerance, bottomWallSize + tolerance, bottomHeight + boxHeight -
        electronicCaseCover
    ])
    {
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
            gridOuter - electronicCaseCoverMagnetHolderThickness - electronicCaseCoverMagnetThickness,
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

module ReedPin()
{
    metalPlateDia = metalPlateRadius * 2;
    cutoutWidthH = metalPlateDia - 2 * reedPinBorder;
    cutoutWidthV = reedPinWidth - 2 * reedPinBorder;
    cutoutHeight = reedPinHeight - reedThickness - reedPinBorder;

    difference()
    {
        // Reed boxHeight - metalPlateThicknesspin
        cube([ metalPlateDia, reedPinWidth, reedPinHeight ]);

        // Reed
        translate([
            -reedOffset, reedPinWidth / 2 - reedThickness / 2, reedPinHeight -
            reedThickness
        ]) cube([ metalPlateDia, reedThickness, reedThickness + c0 ]);

        // Wire
        translate([
            0, reedPinWidth / 2 - reedWireThickness / 2, reedPinHeight - reedThickness
        ]) cube([ reedPinWidth + c0, reedWireThickness, reedThickness + c0 ]);

        // Cutouts
        translate([ reedPinBorder, -c0, -c0 ])
            cube([ cutoutWidthH, reedPinWidth + 2 * c0, cutoutHeight + c0 ]);
        translate([ -c0, reedPinBorder, -c0 ])
            cube([ reedPinWidth + 2 * c0, cutoutWidthV, cutoutHeight + c0 ]);
        translate([ reedPinBorder, -c0, -c0 ])
            cube([ cutoutWidthH, reedPinBorder + c0, reedPinHeight + 2 * c0 ]);
        translate([ reedPinBorder, reedPinWidth - reedPinBorder + c0, -c0 ])
            cube([ cutoutWidthH, reedPinBorder + c0, reedPinHeight + 2 * c0 ]);
    }

    if (renderMetalPlate) {
        translate([
            metalPlateRadius, reedPinWidth / 2,
            reedPinHeight
        ])
            cylinder(h = metalPlateThickness, r = metalPlateRadius);
    }
}

if (!renderPrintable) {
    if (renderTopBoard) {
        translate([
            fieldBorder + tolerance + bottomWallSize + tolerance,
            fieldBorder + tolerance + bottomWallSize + tolerance,
            boxHeight + bottomHeight - top -
            topBoardHeight
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
            bottomWallSize + tolerance, bottomWallSize + tolerance,
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

    if (renderReedPins) {
        cut4(
            [
                gridOuter + bottomWallSize * 2 + tolerance * 2,
                gridOuter + bottomWallSize * 2 + tolerance * 2, bottomHeight +
                boxHeight
            ],
            cutPartsSize)
            translate([
                bottomWallSize + tolerance + fieldBorder,
                bottomWallSize + tolerance + fieldBorder,
                bottomHeight
            ])
        {
            eachGrid()
            {
                translate([
                    fieldSize / 2 - metalPlateRadius,
                    fieldSize / 2 - reedPinWidth / 2,
                    0
                ]) ReedPin();
            }
        }
    }

    translate([ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCase)
    {
        translate([ fullOuterBoard - c0, 0, 0 ]) cut2(
            [ electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight ],
            cutPartsSize) ElectronicCase();
    };

    translate(
        [ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCaseCover)
    {
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
            bottomWallSize + tolerance + 40 + gridOuter * 2, bottomHeight -
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

    if (renderReedPins) {
        translate([ fullOuterBoard + electronicCaseWidth + 50, 0, 0 ]) cut4(
            [
                gridOuter + bottomWallSize * 2 + tolerance * 2,
                gridOuter + bottomWallSize * 2 + tolerance * 2, bottomHeight +
                boxHeight
            ],
            cutPartsSize)
            translate([
                bottomWallSize + tolerance + 3 * fieldBorder + tolerance,
                bottomWallSize + tolerance, 0
            ])
        {
            eachGrid()
            {
                translate([
                    fieldSize / 2 - reedPinWidth / 2, fieldSize / 2 - reedPinWidth / 2, 0
                ]) translate([ 0, reedPinWidth, reedPinHeight ]) rotate([ 180, 0, 0 ]) ReedPin();
            }
        }
    }

    translate([ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCase)
    {
        translate([ fullOuterBoard - c0, 0, 0 ]) cut2(
            [ electronicCaseWidth + c0, fullOuterBoard, bottomHeight + boxHeight ],
            cutPartsSize) ElectronicCase();
    };

    translate(
        [ cutParts ? cutPartsSize : 0, 0, 0 ]) if (renderElectronicCaseCover)
    {
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