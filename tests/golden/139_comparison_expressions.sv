module m;
  int a, b, c;
  initial begin
    a  =  b  ==  c;
    a  =  b  !=  c;
    a  =  b  ===  c;
    a  =  b  !==  c;
    a  =  b  <  c;
    a  =  b  >  c;
    a  =  b  <=  c;
    a  =  b  >=  c;
  end
endmodule
// expected -----
module m;
  int a, b, c;
  initial begin
    a = b == c;
    a = b != c;
    a = b === c;
    a = b !== c;
    a = b < c;
    a = b > c;
    a = b <= c;
    a = b >= c;
  end
endmodule
