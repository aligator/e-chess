renderTop=true;
renderGrid=true;
renderBottom=true;

cutParts=false;

// Also must match the led-strip led distance
fieldSize=33;

// Board field count. For a normal chess -> 8.
size=8;

// 3 layers for layer thickness 0.2
top=0.6; 

fieldBorderHeight=0.4; 
fieldBorder=0.4; 

metalPlateHeight=0.2; // The actual plate is 0.3, but 0.2 matches the layer thickness better.
metalPlateRadius=10.4;

wireRadius=1.5;

boxHeight=30;

ledWidth=11;
ledHeight=3;
bottomHeight=5;
bottomGridOverlap=1;

// Additional bottomSize to the size of the grid.
bottomSize=3;


tollerance=0.2;

// Just a constant to make cutouts larger for better preview rendering.
c0=1+0;

$fa=12;
$fs=1;

gridOuter=size * fieldSize + 4*fieldBorder + 2*tollerance;
cutPartsSize = cutParts ? 10 : 0;

module eachGrid() { 
    for ( i = [0:1:size-1]) {
        for ( j = [0:1:size-1]) {
            translate([i*fieldSize, j*fieldSize, 0])
            children(); 
        }
    }
} 

module cut4(partSize, gap) {
    if (gap == 0) {
        // Fast path to avoid unneeded rendering
        children();
    } else {
        for ( i = [0:1:1]) {
            for ( j = [0:1:1]) {
                translate([i*partSize[0]/2 + i*gap, j*partSize[1]/2 + j*gap, 0])
                intersection() {
                    translate([
                        -i*partSize[0]/2,
                        -j*partSize[1]/2
                    ]) children();
                    cube([
                        partSize[0]/2, 
                        partSize[1]/2, 
                        partSize[2]
                    ]);
                }
            }
        }
    }
}

module Field() {
    difference() {
        cube([fieldSize + fieldBorder*2, fieldSize + fieldBorder*2, top + fieldBorderHeight]);
        translate([fieldBorder, fieldBorder, top]) cube([fieldSize, fieldSize, fieldBorderHeight+c0]);
        translate([fieldSize/2, fieldSize/2, -c0]) cylinder(d=metalPlateRadius, h=metalPlateHeight+c0);
    };
}

module Top() {
    eachGrid() {
        Field();
    }
}

module Grid() {
    translate([fieldBorder + tollerance, fieldBorder + tollerance, 0]) 
    eachGrid() {
        difference() {
            cube([fieldSize + 2*fieldBorder, fieldSize + 2*fieldBorder, boxHeight]);
            translate([fieldBorder, fieldBorder, -c0]) cube([fieldSize, fieldSize, boxHeight+c0*2]);

            // wires
            translate([
                wireRadius + fieldBorder*2, 
                fieldSize+c0, 
                wireRadius + fieldBorder*2,
            ]) 
            rotate([90, 0, 0])
            cylinder(h = fieldSize+c0*2, r = wireRadius);

            translate([
                -c0, 
                wireRadius + fieldBorder*2, 
                wireRadius + fieldBorder*2 + wireRadius*2,
            ]) 
            rotate([90, 0, 90])
            cylinder(h = fieldSize+c0*2, r = wireRadius);
        }
    };

    translate([0, 0, 0]) 
    difference() {
        cube([gridOuter, gridOuter, boxHeight+fieldBorder*2]);

        // inner hole
        translate([fieldBorder*2 + tollerance, fieldBorder*2 + tollerance, -c0]) 
        cube([size * fieldSize, size * fieldSize, boxHeight+fieldBorder*2 + c0*2]);

        // cut out, where the top part goes
        translate([fieldBorder, fieldBorder, boxHeight])
        cube([size * fieldSize + 2*fieldBorder + 2*tollerance, size * fieldSize + 2*fieldBorder + 2*tollerance, fieldBorder+c0]);
    }
}

module BottomElectronic() {
    for ( i = [0:1:size-1]) {
        // Led strips
        translate([fieldBorder, i*fieldSize + bottomSize+tollerance + fieldSize/2 - ledWidth/2, bottomHeight-bottomGridOverlap-ledHeight])
        cube([gridOuter+bottomSize*2+c0, ledWidth, ledHeight+bottomGridOverlap+c0]);
    }

    // Add hole for the wires of the reeds
    translate([gridOuter-fieldSize, bottomSize+tollerance+fieldSize/2+ledWidth/2+fieldBorder + fieldSize*(size-1), bottomHeight-bottomGridOverlap-ledHeight]) 
    cube([fieldSize+bottomSize*2+tollerance+c0, ledHeight, ledHeight+c0]);
}

module Bottom() {
    difference() {
        cube([gridOuter + 2*bottomSize + 2*tollerance, gridOuter + 2*bottomSize + 2*tollerance, bottomHeight]);
        translate([bottomSize, bottomSize, bottomHeight-bottomGridOverlap]) 
        cube([gridOuter + 2*tollerance, gridOuter + 2*tollerance, bottomGridOverlap+c0]);

        BottomElectronic();
    }
}

if (renderTop) {
    translate([fieldBorder + tollerance + bottomSize+tollerance, fieldBorder + tollerance + bottomSize+tollerance, boxHeight+bottomHeight-bottomGridOverlap])
    cut4([
        fieldSize*size + 2*fieldBorder,
        fieldSize*size + 2*fieldBorder,
        bottomHeight+boxHeight
    ], cutPartsSize)
    Top();
}

if (renderGrid) {
    // Grid, including the wiring for the reed contacts.
    // Open at the bottom, to allow easy wiring.
    translate([bottomSize+tollerance, bottomSize+tollerance, bottomHeight-bottomGridOverlap]) 
    cut4([
        gridOuter,
        gridOuter,
        bottomHeight+boxHeight
    ], cutPartsSize)
    Grid();
}

if (renderBottom) {  
    // The bottom embeds the led strip.
    cut4([
        gridOuter + bottomSize*2 + tollerance*2,
        gridOuter + bottomSize*2 + tollerance*2,
        bottomHeight+boxHeight
    ], cutPartsSize)
    Bottom();
}