module m;
  int a, b, c, d;
  initial begin
    a  =  b  +  c;
    a  =  b  -  c;
    a  =  b  *  c;
    a  =  b  /  c;
    a  =  b  %  c;
    a  =  b  &  c;
    a  =  b  |  c;
    a  =  b  ^  c;
    a  =  b  &&  c;
    a  =  b  ||  c;
    a  =  b  <<  2;
    a  =  b  >>  3;
  end
endmodule
// expected -----
module m;
  int a, b, c, d;
  initial begin
    a = b + c;
    a = b - c;
    a = b * c;
    a = b / c;
    a = b % c;
    a = b & c;
    a = b | c;
    a = b ^ c;
    a = b && c;
    a = b || c;
    a = b << 2;
    a = b >> 3;
  end
endmodule
