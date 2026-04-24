module m;
  assign z = (c1)?v1:(c2)?v2:v3;
endmodule
// expected -----
module m;
  assign z = (c1) ? v1 : (c2) ? v2 : v3;
endmodule
