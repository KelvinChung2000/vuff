module m;
  initial   begin
    if (x   inside  { 1, 2, [3:5] })  y = 1;
    if (   a    inside { 100 } )  y = 2;
  end
endmodule
// expected -----
module m;
  initial begin
    if (x inside { 1, 2, [3:5] }) y = 1;
    if (a inside { 100 }) y = 2;
  end
endmodule
