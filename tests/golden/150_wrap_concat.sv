module m;
  wire [7:0] data;
  assign data = {a,
                 b,
                 c};
  assign other = {x, y, z};
endmodule
// expected -----
module m;
  wire [7:0] data;
  assign data = {
    a,
    b,
    c
  };
  assign other = {x, y, z};
endmodule
