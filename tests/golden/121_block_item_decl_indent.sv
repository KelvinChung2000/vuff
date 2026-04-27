module m;
  initial begin
    int   x = 0;
    bit   [3:0]  y;
    automatic int   z;
    x = 1;
    y = 4'b1010;
  end
endmodule
// expected -----
module m;
  initial begin
    int x = 0;
    bit [3:0] y;
    automatic int z;
    x = 1;
    y = 4'b1010;
  end
endmodule
