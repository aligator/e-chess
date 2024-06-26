renderTop=false;
renderGrid=true;
renderBottom=true;

// Also must match the led-strip led distance
fieldSize=33;

// Board field count. For a normal chess -> 8.
size=8;

top=0.4; // 2 layers for layer thickness 0.2

fieldBorderHeight=0.4; 
fieldBorder=0.4; 

metalPlateHeight=0.2; // The actual plate is 0.3, but 0.2 matches the layer thickness better.
metalPlateRadius=10.4;

reedWidth=3;

wireRadius=2;

boxHeight=30;

// Just a constant to make cutouts larger for better preview rendering.
c0=1+0;

$fa=12;
$fs=1;

module eachGrid() { 
    for ( i = [0:1:size-1]) {
        for ( j = [0:1:size-1]) {
            translate([i*fieldSize, j*fieldSize, 0])
            children(); 
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
    eachGrid() {
        difference() {
            cube([fieldSize, fieldSize, boxHeight]);
            translate([fieldBorder, fieldBorder, -c0]) cube([fieldSize - fieldBorder*2, fieldSize - fieldBorder*2, boxHeight+c0*2]);

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
    }

    translate([-fieldBorder, -fieldBorder, 0]) 
    difference() {
        cube([size * fieldSize + 2*fieldBorder, size * fieldSize + 2*fieldBorder, boxHeight+fieldBorder*2]);

        translate([fieldBorder*2, fieldBorder*2, -c0]) 
        cube([size * fieldSize-fieldBorder, size * fieldSize-fieldBorder, boxHeight+fieldBorder*2 + c0*2]);

        translate([fieldBorder, fieldBorder, boxHeight])
        cube([size * fieldSize, size * fieldSize, fieldBorder+c0]);
    }
}

if (renderTop) {
    translate([0, 0, boxHeight]) Top();
}

if (renderGrid) {
    // Grid, including the wiring for the reed contacts.
    // Open at the bottom, to allow easy wiring.
    Grid();
}

if (renderBottom) {
    // The bottom embeds the led strip.

}