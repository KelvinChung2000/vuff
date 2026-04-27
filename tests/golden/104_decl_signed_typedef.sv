module m;
  wire    signed   [7:0]   sx;
  typedef logic [7:0]  byte_t;
  byte_t   data;
endmodule
// expected -----
module m;
  wire signed [7:0] sx;
  typedef logic [7:0] byte_t;
  byte_t data;
endmodule
