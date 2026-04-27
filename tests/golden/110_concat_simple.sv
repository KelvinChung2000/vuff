module m;
  wire   [7:0]   x  =  {a, b, c};
  wire   [15:0]  y  =  {{2{a}},  {3{b}}};
  initial   begin
    {a, b}   =   c;
  end
endmodule
// expected -----
module m;
  wire [7:0] x = {a, b, c};
  wire [15:0] y = {{2{a}}, {3{b}}};
  initial begin
    {a, b} = c;
  end
endmodule
