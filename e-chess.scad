// Also must match the led-strip led distance
fieldSize=33;
fieldHeight=30;

// The inner border is twice as thick
fieldBorder=1; 
stripWidth=11;
stripHeight=3;

wireDiameter=2;
wireGap=3;

diodeWidth=2;
diodeLength=6;
diodeGap=2;
diodeWire=3;

// Board field count. For a normal chess -> 8.
size=8;

topScale=0.98;
top=0.4; // 2 layers for layer thickness 0.2

// Just a constant to make cutouts larger for better prview rendering
c0=1+0;

// cutOffset is cut from the bottom.
module basicField(cutOffset) {
    translate([0, 0, cutOffset])
    difference() {
        cube([fieldSize, fieldSize, fieldHeight-cutOffset]);
        translate([fieldBorder*2, fieldBorder*2, -c0])
            cube([fieldSize-fieldBorder*4, fieldSize-fieldBorder*4, fieldHeight-cutOffset+c0+c0]);
        translate([fieldBorder, fieldBorder, fieldHeight-cutOffset-fieldBorder])
            cube([fieldSize-fieldBorder*2, fieldSize-fieldBorder*2, fieldBorder+c0]);
    };
}

module ledStrip() {
    translate([-c0, fieldSize/2-stripWidth/2, -c0]) cube([fieldSize+c0*2, stripWidth, stripHeight+c0*2]);
}

module wire(size) {
    hull() {
        cylinder(d=wireDiameter, h=size+c0*2);
        translate([-wireDiameter/2, -wireDiameter, 0])
            cube([wireDiameter, wireDiameter, size+c0*2]);
    };
}

module fieldElectronic() {
    // column wire 
    translate([wireGap, -c0, stripHeight-wireDiameter/2])
        rotate([-90, 0, 0]) wire(fieldSize);
    
    // row wire
    translate([-c0, wireGap, stripHeight-wireDiameter/2])
        rotate([-90, 0, -90]) wire(fieldSize);
    
    // diode
    translate([wireGap + wireDiameter/2 + diodeGap, wireGap + diodeGap + wireDiameter/2, stripHeight-diodeWidth])
    cube([diodeLength, diodeWidth, diodeWidth+c0]);
    // wire for the diode
    translate([wireGap + wireDiameter/2 + diodeGap + diodeLength, wireGap + diodeGap + wireDiameter/2 + diodeWidth/2, stripHeight-wireDiameter/2])
        rotate([-90, 0, -90]) wire(diodeWire);
    translate([wireGap + wireDiameter*2 + diodeGap + diodeLength + diodeWire, wireGap + diodeGap + wireDiameter + diodeWidth/2, stripHeight-wireDiameter/2])
        rotate([-90, 0, 180]) wire(diodeGap);
}

module fieldBottom() {
    difference() {
        cube([fieldSize, fieldSize, stripHeight]);
        ledStrip();
        fieldElectronic();
    }
}

module field() {
    basicField(stripHeight);
    fieldBottom();
}

module top() {
    cube([fieldSize - fieldBorder*2, fieldSize - fieldBorder*2, top]);
}

for ( i = [0:1:size-1]) {
    for ( j = [0:1:size-1]) {
        translate([i*fieldSize, j*fieldSize, 0])
        field();
        
        translate([-i*fieldSize - fieldBorder, j*fieldSize, 0])
        scale([topScale, topScale, 1]) mirror([1, 0, 0]) top();
    }
}