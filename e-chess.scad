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

// Just a constant to make cutouts larger for better preview rendering.
c0=1+0;

module field() {
    difference() {
        cube([fieldSize + fieldBorder*2, fieldSize + fieldBorder*2, top + fieldBorderHeight]);
        translate([fieldBorder, fieldBorder, top]) cube([fieldSize, fieldSize, fieldBorderHeight+c0]);
        translate([fieldSize/2, fieldSize/2, -c0]) cylinder(d=metalPlateRadius, h=metalPlateHeight+c0);
    };
}

for ( i = [0:1:size-1]) {
    for ( j = [0:1:size-1]) {
        translate([i*fieldSize, j*fieldSize, 0])
        field();
    }
}