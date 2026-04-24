module m;
  logic [31:0] data;
  logic [7:0] byte0;
  assign byte0 = data [ 7 : 0 ];
endmodule
// expected -----
module m;
  logic [31:0] data;
  logic [7:0] byte0;
  assign byte0 = data[7 : 0];
endmodule
