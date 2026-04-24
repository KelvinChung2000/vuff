module m(
inout wire a,
input wire b,
output logic c
);
endmodule
// expected -----
module m (
  inout  wire  a,
  input  wire  b,
  output logic c
);
endmodule
