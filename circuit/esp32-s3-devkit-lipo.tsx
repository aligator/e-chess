export default () => (
  <board width="50mm" height="60mm">
    {/* Mounting Holes */}
    <platedhole shape="circle" holeDiameter={3.2} outerDiameter={6} pcbX={-20} pcbY={-25} />
    <platedhole shape="circle" holeDiameter={3.2} outerDiameter={6} pcbX={20} pcbY={-25} />
    <platedhole shape="circle" holeDiameter={3.2} outerDiameter={6} pcbX={-20} pcbY={25} />
    <platedhole shape="circle" holeDiameter={3.2} outerDiameter={6} pcbX={20} pcbY={25} />

    {/* Reference Designator */}
    <silkscreentext text="U1" pcbX={0} pcbY={-28} layer="top" />
  </board>
) 