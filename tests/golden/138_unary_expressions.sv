module m;
  int a, b;
  initial begin
    a  =  -b;
    a  =  ~b;
    a  =  !b;
    a  =  &b;
    a  =  |b;
    a  =  ^b;
  end
endmodule
// expected -----
module m;
  int a, b;
  initial begin
    a = -b;
    a = ~b;
    a = !b;
    a = &b;
    a = |b;
    a = ^b;
  end
endmodule
